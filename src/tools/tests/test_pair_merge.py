"""Exercise the standalone pair merger using existing Garmin SDK fixtures and Python wire read-back."""
import csv
import json
import os
import re
import shlex
from pathlib import Path
import subprocess
import tempfile
import unittest

from fit_samples import ROOT, fixture, rows


def build_cli(crate, name):
    manifest = ROOT / "src/rust" / crate / "Cargo.toml"
    log = subprocess.check_output(["cargo", "build", "--locked", "--release", "--bin", name,
                                   "--manifest-path", str(manifest), "--message-format=json"], cwd=ROOT, text=True)
    return next(msg["executable"] for msg in map(json.loads, log.splitlines())
                if msg.get("executable") and msg["target"]["name"] == name)


class PairMerge(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.cli = build_cli("pair-merge", "fit-merge")
        cls.inputs = ROOT / "src/tools/tests/fixtures"
        cls.bike = cls.inputs / "streams/bike.fit"
        cls.band = cls.inputs / "activities/cirqa.fit"

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        config = self.dir / ".config/fit-fusion"
        config.mkdir(parents=True)
        (config / "config.json").write_text("deliberately invalid")
        self.env = dict(os.environ, HOME=str(self.dir), XDG_CONFIG_HOME=str(self.dir / ".config"), PATH="")

    def invoke(self, *args, expected=0):
        result = subprocess.run([self.cli, *map(str, args)], cwd=self.dir, env=self.env,
                                capture_output=True, text=True)
        self.assertEqual(result.returncode, expected, result.stderr)
        return result

    def merge(self, primary, overlay, out, *fields, expected=0):
        args = [word for name in fields for word in ("--stream", name)]
        return self.invoke("--overlay", overlay, *args, "--primary", primary, "--out", out,
                           "--json", expected=expected)

    def test_help_skill_and_failure_before_io(self):
        self.assertIn("fit-merge 0.2.4", self.invoke("--help").stdout)
        self.assertEqual(self.invoke("--skill").stdout, self.invoke("--skill", "index").stdout)
        self.assertIn("fit-merge --skill merge", self.invoke("--skill").stdout)
        self.assertIn("--primary", self.invoke("--skill", "merge").stdout)
        self.invoke("--skill", "unknown", expected=2)
        self.invoke("--primary", "absent.fit", expected=2)
        self.invoke("--bogus", expected=2)
        self.invoke("--primary", self.bike, "--overlay", self.band, "--out", "x.fit", expected=2)
        self.invoke("--primary", self.bike, "--overlay", self.band, "--overlay", self.bike,
                    "--out", "x.fit", "--stream", "record.power", expected=2)
        self.assertFalse((self.dir / "x.fit").exists())

    def test_preflight_guide_inspects_both_inputs(self):
        for topic in ("index", "merge"):
            self.assertIn("fit-merge --skill preflight", self.invoke("--skill", topic).stdout)
        guide = self.invoke("--skill", "preflight")  # Empty PATH, no input files.
        self.assertEqual(guide.stderr, "")
        inspector = build_cli("inspector", "fit-inspector")
        sources = {"primary.fit": self.inputs / "activities/rider.fit", "overlay.fit": self.band}
        for name, source in sources.items():
            (self.dir / name).write_bytes(source.read_bytes())
        before = set(self.dir.iterdir())
        coverage, totals = {}, {}
        for block in re.findall(r"```sh\n(.*?)\n```", guide.stdout, re.S):
            for command in block.splitlines():
                program, action, name = shlex.split(command)
                self.assertEqual(program, "fit-inspector")
                output = subprocess.check_output([inspector, action, name], cwd=self.dir,
                                                 env=self.env, text=True, stderr=subprocess.PIPE)
                if action == "inspect":
                    sampling = json.loads(output)["members"][0]["sampling"]
                    hr = next(row for row in sampling if row["name"] == "record.heart_rate")
                    coverage[name] = (hr["sample_count"], hr["message_count"])
                else:
                    self.assertEqual(action, "raw")
                    layouts = {}
                    for frame in map(json.loads, output.splitlines()):
                        if frame.get("global_id") != 18:
                            continue
                        if frame["kind"] == "definition":
                            layouts[frame["local"]] = frame["fields"]
                        elif frame["kind"] == "data":
                            totals[name] = next(v for f, v in zip(layouts[frame["local"]], frame["values"])
                                                if f["id"] == 11 and "developer_index" not in f)
        self.assertEqual(coverage, {"primary.fit": (5, 6), "overlay.fit": (6, 6)})
        self.assertEqual(totals, {"primary.fit": 6, "overlay.fit": 2})
        self.assertEqual(set(self.dir.iterdir()), before)
        for name, source in sources.items():
            self.assertEqual((self.dir / name).read_bytes(), source.read_bytes())

    def test_skill_recipes_with_sdk_fixtures(self):
        base = self.inputs / "activities/rider.fit"
        for name, source in [("primary.fit", base), ("overlay.fit", self.band), ("power.fit", self.bike)]:
            (self.dir / name).write_bytes(source.read_bytes())
        guide = self.invoke("--skill", "merge").stdout
        for recipe in re.findall(r"```sh\n(.*?)\n```", guide, re.S):
            prog, *args = shlex.split(recipe)
            self.assertEqual(prog, "fit-merge")
            result = self.invoke(*args)
            self.assertTrue((self.dir / json.loads(result.stdout)["artifacts"]["fit"]).is_file())
        bike_hr = [110, 111, 255, 114, 114, 115]
        band_hr = [109, 110, 113, 113, 114, 115]
        for name, expected_hr, expected_total in [
            ("ride.fit", bike_hr, 2),
            ("ride-with-hr.fit", band_hr, 6),
            ("ride-with-power.fit", bike_hr, 2),
        ]:
            with self.subTest(output=name):
                messages = list(rows(self.dir / name))
                self.assertEqual([r[None, 3] for n, r in messages if n == 20], expected_hr)
                session, = (r for n, r in messages if n == 18)
                self.assertEqual((session[None, 11], session[None, 196]), (expected_total, 1))
        power = self.dir / "ride-with-power.fit"
        self.assertEqual([r[None, 7] for n, r in rows(power) if n == 20],
                         [0, 120, 65535, 250, 180, 65535])
        third = self.dir / "third.fit"
        self.merge(self.dir / "ride-with-hr.fit", base, third, "record.heart_rate")
        self.assertEqual([r[None, 3] for n, r in rows(third) if n == 20], bike_hr)

    def test_arrays_and_developer_collisions(self):
        out = self.dir / "array.fit"
        backup = self.inputs / "streams/backup.fit"
        self.merge(self.bike, backup, out, "record.left_power_phase")
        self.assertEqual([r[None, 69] for n, r in rows(out) if n == 20], [[32, 64]] * 6)
        # Alter the two session values independently and keep private/unselected payloads.
        primary = self.dir / "base.fit"
        primary.write_bytes(fixture(True, [110] * 6, metabolic=4))
        # The source is the Garmin-SDK fixture with native total 2 and metabolic 1.
        for fields in [("session.total_calories",), ("session.total_calories", "session.metabolic_calories")]:
            dest = self.dir / (str(len(fields)) + ".fit")
            self.merge(primary, self.band, dest, *fields)
            expected = list(rows(primary))
            for message, values in expected:
                if message == 18:
                    values[None, 11] = 2
                    if "session.metabolic_calories" in fields:
                        values[None, 196] = 1
            self.assertEqual(list(rows(dest)), expected)

    def test_absent_stream_and_destination_protection(self):
        source = self.dir / "primary.fit"
        original = self.bike.read_bytes()
        source.write_bytes(original)
        out = self.dir / "ride.fit"
        result = self.merge(source, self.band, out, "record.heart_rate", "record.power", expected=3)
        self.assertIn("record.power", result.stderr)
        self.assertFalse(out.exists())
        self.merge(source, self.band, source, "record.heart_rate", expected=3)
        out.symlink_to(source)
        self.merge(source, self.band, out, "record.heart_rate", expected=3)
        self.assertTrue(out.is_symlink())
        self.assertEqual(source.read_bytes(), original)

    @unittest.skipUnless((ROOT / "data/index/rides.csv").is_file(), "requires private ride fixtures")
    def test_six_real_rides_with_whole_stream_overwrite(self):
        with (ROOT / "data/index/rides.csv").open() as index:
            for ride in csv.DictReader(index):
                with self.subTest(date=ride["ride_date"]):
                    primary, overlay = ROOT / ride["bryton_fit"], ROOT / ride["garmin_fit"]
                    out = self.dir / (ride["ride_date"] + ".fit")
                    self.merge(primary, overlay, out, "record.heart_rate", "session.total_calories")
                    heart = {row[None, 253]: row.get((None, 3), 255)
                             for number, row in rows(overlay) if number == 20}
                    expected = list(rows(primary))
                    for number, row in expected:
                        if number == 20:
                            row[None, 3] = heart.get(row[None, 253], 255)
                        elif number == 18:
                            row[None, 11] = int(ride["garmin_fit_total_kcal"])
                    self.assertEqual(list(rows(out)), expected)
