"""M4-2 / R08. Run the standalone CLI against shared Garmin SDK and local ride fixtures."""
import csv
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = ROOT / "src/rust/merger/tests/fixtures"
REAL = ROOT / "data/sources/bryton/fit"


class QueryFiles(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build = subprocess.check_output([
            "cargo", "build", "--locked", "--release", "--manifest-path",
            str(ROOT / "src/rust/query-files/Cargo.toml"), "--message-format=json",
        ], cwd=ROOT, text=True)
        cls.cli = next(msg["executable"] for msg in map(json.loads, build.splitlines())
                       if msg.get("executable") and msg["target"]["name"] == "fit-query-files")

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.tmp = Path(tmp.name)
        self.env = dict(os.environ, HOME=str(self.tmp), PATH="", XDG_CONFIG_HOME=str(self.tmp))
        self.out = self.tmp / "selected"

    def invoke(self, *args, code=0):
        run = subprocess.run([self.cli, *map(str, args)], cwd=self.tmp, env=self.env,
                             capture_output=True, text=True)
        self.assertEqual(run.returncode, code, (args, run.stdout, run.stderr))
        return run.stdout if code != 2 else run.stderr

    def query(self, source, *args, code=0):
        return json.loads(self.invoke("--dir", source, *args, "--json", code=code))

    def test_offline_help_skills_and_syntax(self):
        self.assertIn("fit-query-files 0.2.1", self.invoke("--help"))
        self.assertIn("Compiler", self.invoke("--version"))
        index = self.invoke("--skill")
        self.assertEqual(index, self.invoke("--skill", "index"))
        for topic in ("list", "read", "export"):
            self.assertIn("--skill " + topic, index)
            self.assertIn("Example", self.invoke(topic, "--help"))
            self.assertIn("fit-query-files --dir", self.invoke("--skill", topic))
        missing = self.tmp / "missing"
        for args in [("--skill", "unknown"), ("list", "--garbage"),
                     ("read", "--id", "bad"), ("list", "--date", "2026-02-30"),
                     ("list", "--date", "2026-11-21", "--from", "2026-11-21"),
                     ("export", "--from", "2026-01-01", "--out", self.out),
                     ("export", "--id", "a"*64, "--date", "2026-11-21", "--out", self.out),
                     ("list", "--from", "2026-12-01", "--until", "2026-11-01")]:
            self.invoke("--dir", missing, *args, code=2)
        self.assertFalse(self.out.exists())

    def test_display_formats_preserve_identity_and_outcomes(self):
        args = ["--dir", FIXTURES, "list", "--device", "267/2205"]
        pretty = self.invoke(*args)
        compact = self.invoke(*args, "--json")
        self.assertEqual(json.loads(pretty), json.loads(compact))
        self.assertGreater(len(pretty.splitlines()), 1)
        self.assertEqual(len(compact.splitlines()), 1)
        self.invoke("--json", *args, "--text", code=2)
        listing = self.invoke(*args, "--text")
        for row in json.loads(pretty)["activities"]:
            self.assertIsNone(row["activity_name"])
            self.assertEqual(row["device_name"], "Bryton Rider 460")
            self.assertIn(row["id"], listing)
        self.assertIn("Bryton Rider 460", listing)
        self.assertIn("Activities  0", self.invoke("--dir", FIXTURES, "list", "--date", "2000-01-01", "--text"))
        self.assertIn("Status  error", self.invoke("--dir", self.tmp / "missing", "list", "--text", code=3))

    def test_list_read_and_export_one_renamed_recording(self):
        source = self.tmp / "rides"
        shutil.copytree(FIXTURES, source)
        rides = self.query(source, "list", "--date", "2024-01-09")["activities"]
        self.assertEqual(len(rides), 3)
        self.assertEqual(len(self.query(source, "list", "--device", "267/2205")["activities"]), 2)
        file = source / "rider.fit"
        original = file.read_bytes()
        ride_id = hashlib.sha256(original).hexdigest()
        self.assertIn(ride_id, [a["id"] for a in rides])
        file.rename(source / "renamed.FIT")
        detail = self.query(source, "read", "--id", ride_id)
        self.assertEqual(detail["activity"]["device"]["name"], "Bryton Rider 460")
        self.assertEqual(Path(detail["activity"]["path"]).name, "renamed.FIT")
        heart = next(s for s in detail["streams"] if s["name"] == "record.heart_rate")
        self.assertEqual(heart["sample_count"], 5)
        for expected in ("exported", "reused"):
            item, = self.query(source, "export", "--id", ride_id, "--out", self.out)["exports"]
            self.assertEqual((item["format"], item["status"], item["sha256"]), ("original", expected, ride_id))
            self.assertEqual(Path(item["path"]).read_bytes(), original)
        self.assertEqual(len(list(self.out.iterdir())), 1)
        self.assertEqual(self.query(source, "list", "--date", "2000-01-01")["activities"], [])
        self.assertEqual(self.query(source, "read", "--id", "0"*64, code=3)["status"], "error")
        self.assertTrue(self.query(self.tmp / "missing", "list", code=3)["errors"])

    def test_batch_preserves_conflict_and_reuses_successful_items(self):
        src = self.tmp / "source"
        shutil.copytree(FIXTURES, src)
        nested = src / "selected"
        nested.mkdir()
        blocked = nested / "rider.fit"
        blocked.write_bytes(b"keep existing destination")
        for expected in ("exported", "reused"):
            batch = self.query(src, "export", "--date", "2024-01-09", "--out", nested, code=3)
            self.assertEqual(batch["status"], "partial")
            self.assertCountEqual([item["status"] for item in batch["exports"]],
                                  ["conflict", expected, expected])
            self.assertEqual(len(batch["errors"]), 1)
            self.assertEqual(Path(batch["errors"][0]["path"]).resolve(), blocked.resolve())
            self.assertEqual(blocked.read_bytes(), b"keep existing destination")
            for file in FIXTURES.glob("*.fit"):
                self.assertEqual((src / file.name).read_bytes(), file.read_bytes())

    @unittest.skipUnless((ROOT / "data/index/rides.csv").is_file() and REAL.is_dir(),
                         "requires private rides")
    def test_real_november_export_and_october_midnight(self):
        with (ROOT / "data/index/rides.csv").open() as index:
            originals = {r["ride_date"]: ROOT / r["bryton_fit"] for r in csv.DictReader(index)}
        listed = self.query(REAL, "list", "--date", "2026-10-28")["activities"]
        self.assertEqual([Path(r["path"]) for r in listed], [originals["2026-10-28"]])
        self.assertEqual(self.query(REAL, "list", "--date", "2026-10-29")["activities"], [])
        nov21, = self.query(REAL, "list", "--date", "2026-11-21")["activities"]
        detail = self.query(REAL, "read", "--id", nov21["id"])
        self.assertEqual(detail["activity"]["start_utc"], "2026-11-21T06:11:51Z")
        self.assertIn("record.heart_rate", [s["name"] for s in detail["streams"]])
        expected = {f.name: f.read_bytes() for day, f in originals.items() if day.startswith("2026-11")}
        self.assertEqual(len(expected), 3)
        for status in ("exported", "reused"):
            result = self.query(REAL, "export", "--from", "2026-11-01", "--until", "2026-12-01",
                                "--out", self.out)
            self.assertCountEqual([Path(i["path"]).name for i in result["exports"]], expected)
            for item in result["exports"]:
                file = Path(item["path"])
                self.assertEqual(file.read_bytes(), expected[file.name])
                self.assertEqual(item["sha256"], hashlib.sha256(expected[file.name]).hexdigest())
                self.assertEqual(item["status"], status)
