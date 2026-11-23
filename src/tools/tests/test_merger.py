"""Verify the production offline Rust merger with the independent Python FIT reader."""
import csv
import hashlib
import json
from pathlib import Path
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
        for r in rides:
            date=r["ride_date"]
            with self.subTest(date=date):
                b,g=ROOT/r["bryton_fit"],ROOT/r["garmin_fit"]
                original=list(rows(b))
                garmin={x[None,253]:x.get((None,3)) for n,x in rows(g) if n==20}
                mode,count=EXPECTED[date]
                fit,report,_=self.merge(g,b,date)
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
