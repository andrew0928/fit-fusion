"""Standalone Garmin CLI acceptance. Device and wire expectations use the existing SDK fixture."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
FIXTURE = ROOT / "src/tools/tests/fixtures/activities/cirqa.fit"


class GarminQuery(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build = subprocess.check_output([
            "cargo", "build", "--locked", "--release", "--manifest-path",
            str(ROOT / "src/rust/query-garmin/Cargo.toml"), "--message-format=json",
        ], cwd=ROOT, text=True)
        cls.cli = next(msg["executable"] for msg in map(json.loads, build.splitlines())
                       if msg.get("executable") and msg["target"]["name"] == "fit-query-garmin")

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.cache = self.root / "cache"
        self.env = {"HOME": str(self.root), "PATH": "", "LANG": "C"}

    def invoke(self, *args, code=0):
        r = subprocess.run([self.cli, *map(str, args)], cwd=self.root, env=self.env,
                           capture_output=True, text=True, timeout=30)
        self.assertEqual(r.returncode, code, (args, r.stdout, r.stderr))
        return r.stdout if code != 2 else r.stderr

    def test_agent_help_topics_and_errors_before_network(self):
        self.assertIn("fit-query-garmin 0.2.4", self.invoke("--help"))
        self.assertIn("Compiler", self.invoke("--version"))
        index = self.invoke("--skill")
        self.assertEqual(index, self.invoke("--skill", "index"))
        for topic in ("auth", "list", "read", "export"):
            self.assertIn("--skill " + topic, index)
            self.assertIn("fit-query-garmin ", self.invoke("--skill", topic))
        for cmd in [("list",), ("read",), ("export",)]:
            self.assertIn("Example", self.invoke(*cmd, "--help"))
        for args in [("--skill", "other"), ("list", "--from", "2026-11-21"),
                     ("list", "--from", "2026-12-02", "--until", "2026-12-01"),
                     ("export", "--offline", "--date", "2026-11-21", "--out", "out"),
                     ("read", "--id", "../123"), ("export", "--out", "out"),
                     ("export", "--date", "2026-01-01", "--id", "1", "--out", "out")]:
            self.invoke(*args, code=2)
        self.assertFalse((self.root / "out").exists())

    def test_only_dotenv_files_supply_credentials(self):
        self.env["GARMIN_PERSONAL_ACCESS_TOKEN"] = "synthetic-env-value"
        path = self.root / "incomplete.env"
        path.write_text("OTHER=synthetic-file-value\n")
        self.invoke("list", "--json", code=4)
        self.invoke("--keychain", "any", "list", code=2)
        self.invoke("auth", "status", code=2)
        args = ["--env-file", path, "list", "--json"]
        output = self.invoke(*args, code=4)
        err = json.loads(output)["error"]
        self.assertIn("--env-file", err["message"])
        self.assertNotIn("synthetic", output)

    def test_offline_read_export_conflict_and_cache_miss(self):
        src = FIXTURE.read_bytes()
        digest = hashlib.sha256(src).hexdigest()
        record = self.cache / "42"
        record.mkdir(parents=True)
        (record / "sha256").write_text(digest)
        original = record / (digest + ".fit")
        original.write_bytes(src)
        args = ["--env-file", self.root / "absent.env", "--cache-dir", self.cache, "--json"]
        ride = json.loads(self.invoke(*args, "read", "--offline", "--id", "42"))
        self.assertEqual(ride["activity"]["device"]["name"], "Garmin CIRQA")
        self.assertEqual(ride["activity"]["device_name"], "Garmin CIRQA")
        self.assertIsNone(ride["activity"]["activity_name"])
        cmd = ["--cache-dir", self.cache, "read", "--offline", "--id", "42"]
        self.assertIn("record.heart_rate", self.invoke(*cmd, "--text"))
        self.assertEqual(json.loads(self.invoke(*cmd)), json.loads(self.invoke(*cmd, "--json")))
        self.invoke("--text", *cmd, "--json", code=2)
        self.assertIn("record.heart_rate", [s["name"] for s in ride["streams"]])
        export = [*args, "export", "--id", "42", "--offline", "--out", "out"]
        for status in ("exported", "reused"):
            item, = json.loads(self.invoke(*export))["exports"]
            self.assertEqual((item["status"], item["cache_hit"], item["sha256"]), (status, True, digest))
            self.assertEqual((self.root / item["path"]).read_bytes(), src)
        dest = self.root / "out/42.fit"
        dest.write_bytes(b"other")
        self.assertEqual(json.loads(self.invoke(*export, code=3))["exports"][0]["status"], "conflict")
        self.assertEqual(dest.read_bytes(), b"other")
        self.assertEqual(original.read_bytes(), src)
        self.assertEqual(json.loads(self.invoke(*args, "read", "--offline", "--id", "43", code=3))["error"]["code"],
                         "cache_miss")
