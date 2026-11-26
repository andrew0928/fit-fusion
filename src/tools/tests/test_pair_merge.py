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

from test_merger import ROOT, fixture, rows


class PairMerge(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        manifest = ROOT / "src/rust/pair-merge/Cargo.toml"
        log = subprocess.check_output(["cargo", "build", "--locked", "--release", "--manifest-path",
                                       str(manifest), "--message-format=json"], cwd=ROOT, text=True)
        cls.cli = next(msg["executable"] for msg in map(json.loads, log.splitlines())
                       if msg.get("executable") and msg["target"]["name"] == "fit-merge")
        cls.inputs = ROOT / "src/rust/merger/tests"
        cls.bike = cls.inputs / "stream-fixtures/bike.fit"
        cls.band = cls.inputs / "fixtures/cirqa.fit"

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
        self.assertIn("fit-merge 0.2.3", self.invoke("--help").stdout)
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

    def test_chained_overwrite_and_csharp_fit_readback(self):
        base = self.inputs / "fixtures/rider.fit"
        first, second = self.dir / "ride.fit", self.dir / "ride-with-power.fit"
        for name, source in [("bike.fit", base), ("band.fit", self.band), ("power.fit", self.bike)]:
            (self.dir / name).write_bytes(source.read_bytes())
        guide = self.invoke("--skill", "merge").stdout
        block, = re.findall(r"```sh\n(.*?)\n```", guide, re.S)
        for line in block.splitlines():
            prog, *args = shlex.split(line)
            self.assertEqual(prog, "fit-merge")
            result = self.invoke(*args)
            self.assertTrue((self.dir / json.loads(result.stdout)["artifacts"]["fit"]).is_file())
        expected_hr = [109, 110, 113, 113, 114, 115]
        for file in [first, second]:
            messages = list(rows(file))
            self.assertEqual([r[None, 3] for n, r in messages if n == 20], expected_hr)
            self.assertEqual(next(r[None, 11] for n, r in messages if n == 18), 2)
        self.assertEqual([r[None, 7] for n, r in rows(second) if n == 20],
                         [0, 120, 65535, 250, 180, 65535])
        third = self.dir / "third.fit"
        self.merge(second, base, third, "record.heart_rate")
        self.assertEqual([r[None, 3] for n, r in rows(third) if n == 20],
                         [110, 111, 255, 114, 114, 115])

    def test_arrays_and_developer_collisions(self):
        out = self.dir / "array.fit"
        backup = self.inputs / "stream-fixtures/backup.fit"
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
