"""Verify the production offline Rust merger with the independent Python FIT reader."""
import csv
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path[:0] = [str(ROOT / "src/python"), str(ROOT / "src/python/tests")]
from fit_fusion.wire import wire_frames
from test_inspect import fit_member, message_definition

START = 0x40000000
EXPECTED = {
    "2026-10-09": ("fill", 78), "2026-10-14": ("fill", 1), "2026-10-28": ("whole", 9632),
    "2026-11-01": ("fill", 34), "2026-11-08": ("whole", 17190), "2026-11-21": ("whole", 14149),
}


def fixture(bryton, rates, start=START):
    n = len(rates)
    maker, product = (267, 2205) if bryton else (1, 5019)
    body = message_definition(0, 0, [(0, 1, 0), (1, 2, 0x84), (2, 2, 0x84)])
    body += struct.pack("<BBHH", 0, 4, maker, product)
    # Existing developer index zero reserves the namespace, including a field numbered 11.
    body += message_definition(1, 207, [(3, 1, 2)]) + bytes([1, 0])
    body += message_definition(1, 206, [(0, 1, 2), (1, 1, 2), (2, 1, 2), (3, 4, 7)])
    body += bytes([1, 0, 11, 2]) + b"old\0"
    body += message_definition(2, 20, [(253, 4, 0x86), (3, 1, 2), (4, 1, 2), (240, 1, 2)])
    for i, hr in enumerate(rates):
        body += struct.pack("<BIBBB", 2, start+i, hr, 0, 42)
    body += message_definition(3, 18, [(253, 4, 0x86), (2, 4, 0x86), (5, 1, 0), (7, 4, 0x86),
                                        (11, 2, 0x84), (196, 2, 0x84)], [(11, 1, 0)])
    body += struct.pack("<BIIBIHHB", 3, start+n-1, start, 2, n*1000, 6 if bryton else 2, 1, 77)
    return fit_member(body)


def rows(path):
    layouts = {}
    for f in wire_frames(path.read_bytes()):
        if f["kind"] == "definition":
            layouts[f["local"]] = f["fields"]
        if f["kind"] == "data":
            yield f["global_id"], {(d.get("developer_index"), d["id"]): v for d, v in zip(layouts[f["local"]], f["values"])}


class Merger(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        log = subprocess.check_output(["cargo", "build", "--release", "--locked", "--message-format=json",
                                       "--manifest-path", str(ROOT / "src/rust/merger/Cargo.toml")], cwd=ROOT, text=True)
        cls.cli = next(m["executable"] for m in map(json.loads, log.splitlines()) if m.get("executable"))

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        self.b = self.dir/"first.fit"
        self.g = self.dir/"second.fit"

    def merge(self, b, g, out="out", expected=0):
        p = subprocess.run([self.cli, "merge", str(b), str(g), "--out", str(self.dir/out), "--json"],
                           text=True, capture_output=True)
        self.assertEqual(p.returncode, expected, p.stderr)
        response = json.loads(p.stdout)
        if expected:
            return response
        report = json.loads(Path(response["artifacts"]["report"]).read_text())
        fit = Path(response["artifacts"]["fit"])
        self.assertEqual(hashlib.sha256(fit.read_bytes()).hexdigest(), report["output_sha256"])
        return fit, report, response

    def test_m14_six_seconds_and_safe_retry(self):
        original = fixture(True, [110, 111, 255, 114, 114, 115])
        self.b.write_bytes(original)
        self.g.write_bytes(fixture(False, [109, 110, 113, 113, 114, 115]))
        fit, report, _ = self.merge(self.b, self.g)
        data = list(rows(fit))
        self.assertEqual([r[None, 3] for n, r in data if n==20], [110,111,113,114,114,115])
        self.assertTrue(all((r[None,4], r[None,240])==(0,42) for n,r in data if n==20))
        session = next(r for n,r in data if n==18)
        self.assertEqual((session[None,11], session[None,196],session[0,11],session[1,11],session[1,196]),
                         (2,1,"4d","0200","0100"))
        self.assertEqual(report["heart_rate"]["samples"], {"bryton":5,"garmin":1,"missing":0})
        self.assertEqual(report["calories"]["active_calories"],1)
        self.assertTrue(self.merge(self.g,self.b)[2]["reused"])
        self.assertEqual(self.b.read_bytes(),original)
        fit.write_bytes(b"existing")
        self.assertEqual(self.merge(self.b,self.g,expected=3)["code"],"output_conflict")
        self.assertEqual(fit.read_bytes(),b"existing")

    def test_configured_directory_and_sdk_fixture_merge(self):
        fresh=self.dir/"private"/"config.json"
        init=[self.cli,"--config",str(fresh),"config","init","--json"]
        self.assertEqual(json.loads(subprocess.check_output(init))["code"],"created")
        self.assertEqual(fresh.stat().st_mode & 0o777,0o600)
        self.assertEqual(fresh.parent.stat().st_mode & 0o777,0o700)
        fresh.write_text("existing config")
        self.assertEqual(json.loads(subprocess.check_output(init))["code"],"already_exists")
        self.assertEqual(fresh.read_text(),"existing config")
        config = ROOT / "docs/config/local-fit.example.json"
        command = [self.cli, "--config", str(config)]
        check = json.loads(subprocess.check_output(command + ["config", "check", "--json"], cwd=self.dir))
        self.assertEqual(check["streams"]["record.heart_rate"], ["bryton", "garmin"])
        fixtures = ROOT / "src/rust/merger/tests/fixtures"
        for source, files in (("bryton", ["rider.fit", "rider-later.fit"]), ("garmin", ["cirqa.fit"])):
            result = json.loads(subprocess.check_output(
                command + ["activities", "list", "--source", source, "--json"], cwd=self.dir))
            self.assertEqual([a["id"] for a in result["activities"]],
                             [hashlib.sha256((fixtures / name).read_bytes()).hexdigest() for name in files])
        self.date_setup()
        shutil.copy(fixtures/"rider.fit", self.primary/"ride.FIT")
        shutil.copy(fixtures/"cirqa.fit", self.secondary/"band.fit")
        batch=self.configured("--date","2024-01-09")
        self.assertEqual(len(batch["rides"]),1)
        artifacts=batch["rides"][0]["artifacts"]
        fit=Path(artifacts["fit"])
        report=json.loads(Path(artifacts["report"]).read_text())
        raw = subprocess.check_output(
            [sys.executable, "-m", "fit_fusion", "raw", str(fit)], cwd=self.dir,
            env={**os.environ, "PYTHONPATH": str(ROOT / "src/python")}, text=True)
        layouts, heart, sessions = {}, [], []
        for line in raw.splitlines():
            frame = json.loads(line)
            if frame["kind"] == "definition":
                layouts[frame["local"]] = frame["fields"]
            elif frame["kind"] == "data":
                fields = {field["id"]: v for field, v in zip(layouts[frame["local"]], frame["values"])
                          if field.get("developer_index") is None}
                if frame["global_id"] == 20:
                    heart.append(fields[3])
                elif frame["global_id"] == 18:
                    sessions.append((fields[11], fields[196]))
        self.assertEqual(heart, [110, 111, 113, 114, 114, 115])
        self.assertEqual(sessions, [(2, 1)])
        self.assertEqual(report["heart_rate"]["samples"], {"bryton": 5, "garmin": 1, "missing": 0})

    def date_setup(self):
        self.conf = self.dir / "config.json"
        self.primary = self.dir / "bikes"
        self.secondary = self.dir / "watches"
        self.primary.mkdir()
        self.secondary.mkdir()
        v = json.loads((ROOT / "docs/config/config.example.json").read_text())
        v["sources"]["bryton"]["directory"] = str(self.primary)
        v["sources"]["garmin"] = {"kind": "fit_directory", "device": "garmin_cirqa", "directory": str(self.secondary)}
        v["output_dir"] = str(self.dir / "dates")
        self.conf.write_text(json.dumps(v))
        return v

    def configured(self, *args, expected=0):
        result = subprocess.run([self.cli, "--config", str(self.conf), "merge", *map(str,args), "--json"],
                                cwd=self.dir, text=True, capture_output=True)
        self.assertEqual(result.returncode, expected, result.stdout + result.stderr)
        return json.loads(result.stdout)

    def test_date_pairing_partial_rerun_ambiguity_and_manual_id(self):
        self.date_setup()
        day = ("--date", "2024-01-09")
        fixtures = ROOT / "src/rust/merger/tests/fixtures"
        scenarios = fixtures.parent / "scenarios"
        self.assertEqual(self.configured(*day)["status"], "no_activities")
        shutil.copy(fixtures/"rider.fit", self.primary/"one.FIT")
        shutil.copy(fixtures/"rider-later.fit", self.primary/"two.fit")
        shutil.copy(fixtures/"cirqa.fit", self.secondary/"first.fit")
        partial = self.configured(*day, expected=3)
        self.assertEqual((partial["status"], [r["code"] for r in partial["rides"]]), ("partial", ["ok", "quality_review"]))
        self.assertEqual(len(list((self.dir/"dates").rglob("merged.fit"))), 1)
        self.assertEqual(Path(partial["rides"][0]["artifacts"]["fit"]).read_bytes(),
                         self.merge(self.primary/"one.FIT", self.secondary/"first.fit")[0].read_bytes())
        (self.primary/"one.FIT").rename(self.primary/"renamed.fit")
        shutil.copy(scenarios/"cirqa-later.fit", self.secondary/"second.fit")
        complete = self.configured(*day)
        self.assertEqual([r["reused"] for r in complete["rides"]], [True, False])
        self.assertEqual(len(list((self.dir/"dates").rglob("merged.fit"))), 2)
        shutil.copy(scenarios/"cirqa-later-alt.fit", self.secondary/"alt.fit")
        partial = self.configured(*day, expected=3)
        good, bad = partial["rides"]
        self.assertTrue(good["reused"])
        self.assertEqual(bad["code"], "ambiguous_match")
        self.assertEqual(len(bad["details"]["candidates"]), 2)
        for item in bad["details"]["candidates"]:
            self.assertEqual(item["session_ratios"], [1,1])
            self.assertEqual(item["record_ratios"], [1,1])
            self.assertGreater(item["end_utc"], item["start_utc"])
        result = self.configured(self.primary/"two.fit", "--garmin-id", bad["details"]["candidates"][0]["activity_id"],
                                 "--out", self.dir/"resolved")
        self.assertTrue(Path(result["artifacts"]["fit"]).is_file())
        self.configured(self.primary/"renamed.fit", "--garmin", "auto", "--out", self.dir/"auto")
        shutil.copy(scenarios/"rider-gps.fit", self.primary/"overlapping.fit")
        shared = self.configured(*day, expected=3)
        self.assertTrue(all(r["code"] == "ambiguous_match" for r in shared["rides"]))

    def test_date_config_changes_fit_and_keeps_original(self):
        v = self.date_setup()
        scenarios = ROOT / "src/rust/merger/tests/scenarios"
        main = self.primary/"original.fit"
        watch = self.secondary/"watch.fit"
        shutil.copy(scenarios/"rider-gps.fit", main)
        shutil.copy(scenarios/"cirqa-sensors.fit", watch)
        day = ("--date", "2024-01-09")
        initial = self.configured(*day)["rides"][0]
        v["merge"]["streams"] = {"record.position_lat":["bryton"], "record.position_long":["bryton"],
                                 "record.cadence": ["garmin","bryton"]}
        self.conf.write_text(json.dumps(v))
        new = self.configured(*day)["rides"][0]
        self.assertNotEqual(new["revision_id"], initial["revision_id"])
        report = json.loads(Path(new["artifacts"]["report"]).read_text())
        self.assertEqual(report["stream_sources"]["record.cadence"][0]["source"], "garmin")
        records = list(rows(Path(new["artifacts"]["fit"])))
        for n, fields in records:
            if n == 20:
                self.assertNotIn((None,13), fields)
                self.assertNotIn((None,3), fields)
            if n == 18:
                for key in [57,11,196]:
                    self.assertNotIn((None,key), fields)
        points = [f for n,f in records if n == 20]
        self.assertEqual(len(points),6)
        self.assertEqual((points[2][None,0],points[2][None,1]),(2**31-1,2**31-1))
        self.assertEqual(points[0][None,4],0)
        self.assertEqual(main.read_bytes(),(scenarios/"rider-gps.fit").read_bytes())
        v["merge"]["streams"]={"record.cadence":["bryton"]}
        self.conf.write_text(json.dumps(v))
        watch.unlink()
        self.secondary.rmdir()
        only=self.configured(*day)["rides"][0]
        self.assertEqual(list(json.loads(Path(only["artifacts"]["report"]).read_text())["inputs"]),["bryton"])

    def test_m04_insufficient_fallback_and_invalid_pair(self):
        self.b.write_bytes(fixture(True,[255]*600+[120]*400))
        self.g.write_bytes(fixture(False,[255]*100+[110]*900))
        self.assertEqual(self.merge(self.b,self.g,expected=3)["code"],"quality_review")
        self.assertFalse((self.dir/"out").exists())
        self.g.write_bytes(fixture(False,[110]*6,start=START+9000))
        self.assertEqual(self.merge(self.b,self.g,expected=3)["code"],"unmatched_activities")
        self.g.write_bytes(self.g.read_bytes()[:-1])
        self.assertEqual(self.merge(self.b,self.g,expected=3)["code"],"invalid_data")

    @unittest.skipUnless((ROOT/"data/index/rides.csv").is_file(), "private ride fixtures unavailable")
    def test_six_real_rides(self):
        with (ROOT/"data/index/rides.csv").open() as f:
            rides = list(csv.DictReader(f))
        config = self.date_setup()
        config["sources"]["bryton"]["directory"]=str(ROOT/"data/sources/bryton/fit")
        config["sources"]["garmin"]["directory"]=str(ROOT/"data/sources/garmin")
        self.conf.write_text(json.dumps(config))
        empty=self.configured("--date","2026-10-15")
        self.assertEqual((empty["status"],empty["rides"]),("no_activities",[]))
        for r in rides:
            date=r["ride_date"]
            with self.subTest(date=date):
                b,g=ROOT/r["bryton_fit"],ROOT/r["garmin_fit"]
                original=list(rows(b))
                garmin={x[None,253]:x.get((None,3)) for n,x in rows(g) if n==20}
                mode,count=EXPECTED[date]
                fit,report,_=self.merge(g,b,date)
                batch=self.configured("--date",date)
                self.assertEqual(len(batch["rides"]),1)
                self.assertEqual(Path(batch["rides"][0]["artifacts"]["fit"]).read_bytes(),fit.read_bytes())
                merged=list(rows(fit))
                rec=[x for n,x in merged if n==20]
                primary=[x for n,x in original if n==20]
                self.assertEqual(len(primary),len(rec))
                for x,y in zip(primary,rec):
                    t=x[None,253]
                    hr=x.get((None,3))
                    if mode=="whole" or hr in (None,0,255):
                        hr=garmin.get(t)
                    self.assertEqual(y.get((None,3)),hr)
                    for key,value in x.items():
                        if key!=(None,3): self.assertEqual(y[key],value)
                self.assertEqual(report["heart_rate"]["samples"]["garmin"],count)
                # Entire private messages and all unrelated messages stay intact.
                self.assertEqual([(n,x) for n,x in original if n not in (18,19,20,206,207)],
                                 [(n,x) for n,x in merged if n not in (18,19,20,206,207)])
                if date=="2026-11-21":
                    self.assertEqual([x["seconds"] for x in report["heart_rate"]["triggers"] if x["reason"]=="stale"],[68,86])
                    self.assertTrue(self.merge(b,g,date)[2]["reused"])
                session=next(x for n,x in merged if n==18)
                idx=report["calories"]["developer_index"]
                for fid,key in [(11,"garmin_fit_total_kcal"),(196,"garmin_fit_metabolic_kcal")]:
                    self.assertEqual(session[None,fid],int(r[key]))
                    self.assertEqual(int.from_bytes(bytes.fromhex(session[idx,fid]),"little"),int(r[key]))
                self.assertEqual(hashlib.sha256(b.read_bytes()).hexdigest(),report["inputs"]["bryton"]["sha256"])
                self.assertEqual(hashlib.sha256(g.read_bytes()).hexdigest(),report["inputs"]["garmin"]["sha256"])
