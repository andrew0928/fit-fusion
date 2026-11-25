"""R10. Convert a localized Health ZIP and independently decode each FIT revision."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from xml.etree import ElementTree as ET
import zipfile
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "src/python"))
from fit_fusion.offline import inspect_wire
from fit_fusion.wire import wire_frames

START = "2026-11-21 23:58:00 +0800"
END = "2026-11-22 00:08:00 +0800"
WATCH = {"device": "<<HKDevice>, name:Apple Watch, model:Watch, hardware:Watch7,1, software:12>"}


def archive(path, route=False, duplicate=False):
    health = ET.Element("HealthData")
    for source in ["Watch", "Connect"] + (["Watch"] if duplicate else []):
        attrs = WATCH if source == "Watch" else {}
        workout = ET.SubElement(health, "Workout", workoutActivityType="HKWorkoutActivityTypeCycling",
                                startDate=START, endDate=END, sourceName=source, activityName="同名騎乘", **attrs)
        ET.SubElement(workout, "WorkoutStatistics", type="HKQuantityTypeIdentifierActiveEnergyBurned",
                      sum="73", unit="kcal")
        if source == "Watch":
            r = ET.SubElement(workout, "WorkoutRoute")
            ET.SubElement(r, "MetadataEntry", key="HKExternalUUID", value="route-id-must-not-be-workout-id")
            ET.SubElement(r, "FileReference", path="/workout-routes/test.gpx")
    ET.SubElement(health, "Workout", workoutActivityType="HKWorkoutActivityTypeRunning", startDate=START,
                  endDate=END, sourceName="Watch")
    for source, value, attrs in [("Watch", "98", WATCH), ("Watch", "1", {"device": "hardware:Watch99,9>"}),
                                  ("Connect", "200", {})]:
        ET.SubElement(health, "Record", type="HKQuantityTypeIdentifierHeartRate", sourceName=source,
                      value=value, unit="count/min", startDate=START, endDate=START, **attrs)
    with zipfile.ZipFile(path, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("apple_health_export/匯出.xml", ET.tostring(health))
        z.writestr("apple_health_export/export_cda.xml", "<ClinicalDocument/>")
        if route:
            z.writestr("apple_health_export/workout-routes/test.gpx", """<gpx><trk><trkseg>
<trkpt lat="25.1" lon="121.5"><ele>12.5</ele><time>2026-11-21T15:58:00.125Z</time></trkpt>
</trkseg></trk></gpx>""")


class AppleQuery(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        build = subprocess.check_output(["cargo", "build", "--locked", "--release", "--manifest-path",
                                         str(ROOT / "src/rust/query-apple/Cargo.toml"), "--message-format=json"], cwd=ROOT, text=True)
        cls.binary = next(m["executable"] for m in map(json.loads, build.splitlines()) if m.get("executable"))

    def setUp(self):
        workspace = tempfile.TemporaryDirectory()
        self.addCleanup(workspace.cleanup)
        self.root = Path(workspace.name)
        self.zip = self.root / "health.zip"
        archive(self.zip)

    def run_cli(self, *args, code=0):
        r = subprocess.run([self.binary, *map(str, args)], cwd=self.root, text=True, capture_output=True)
        self.assertEqual(r.returncode, code, (r.stdout, r.stderr))
        return r.stdout if code != 2 else r.stderr

    def query(self, *args, code=0):
        return json.loads(self.run_cli("--archive", self.zip, *args, code=code))

    def test_offline_help_and_syntax(self):
        index = self.run_cli("--skill")
        self.assertEqual(index, self.run_cli("--skill", "index"))
        self.assertIn("Compiler", self.run_cli("--version"))
        for topic in ("list", "read", "export"):
            self.assertIn("--skill " + topic, index)
            self.assertIn("Example", self.run_cli(topic, "--help"))
            self.assertIn("fit-query-apple --archive", self.run_cli("--skill", topic))
        self.run_cli("--skill", "unknown", code=2)
        self.run_cli("list", "--bogus", code=2)
        self.run_cli("--archive", "absent", "list", "--from", "2026-11-22", code=2)
        self.run_cli("--archive", "absent", "list", "--json", "--text", code=2)

    def test_sources_samples_revisions_and_independent_readback(self):
        day = ["--date", "2026-11-21"]
        items = self.query("list", *day)["activities"]
        self.assertEqual(len(items), 2)
        self.assertEqual(len({a["id"] for a in items}), 2)
        self.assertEqual({a["activity_name"] for a in items}, {"同名騎乘"})
        self.assertEqual(self.query("list", "--date", "2026-11-22")["activities"], [])
        a, = self.query("list", *day, "--source-name", "Watch")["activities"]
        self.assertEqual(a["device_name"], "Apple Watch Series 9")
        self.assertIsNone(next(a for a in items if a["source_name"] == "Connect")["device_name"])
        cmd = ["export", "--id", a["id"], "--out", "rides"]
        before = self.zip.read_bytes()
        old, = self.query(*cmd)["exports"]
        self.assertEqual(old["coverage"]["missing"], ["route"])
        self.assertEqual(old["format"], "converted")
        self.assertEqual(self.query(*cmd)["exports"][0]["status"], "reused")
        self.assertEqual(before, self.zip.read_bytes())
        archive(self.zip, route=True)
        new, = self.query(*cmd)["exports"]
        self.assertEqual(old["id"], new["id"])
        self.assertNotEqual(old["path"], new["path"])
        self.assertEqual(new["coverage"]["missing"], [])
        self.assertEqual(new["coverage"]["route_points"], 1)
        for item in (old, new):
            path = self.root / item["path"]
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), item["sha256"])
            messages = inspect_wire(path)["members"][0]["messages"]
            record = next(m for m in messages if m["number"] == 20)
            self.assertEqual(next(f for f in record["fields"] if f["name"] == "heart_rate")["sample_count"], 1)
            self.assertIn(b"Apple Health conversion / Watch", path.read_bytes())
            frames = list(wire_frames(path.read_bytes()))
            defs = {}
            hr = []
            for f in frames:
                if f["kind"] == "definition": defs[f["local"]] = f["fields"]
                if f["kind"] == "data" and f["global_id"] == 20:
                    hr.extend(v for field, v in zip(defs[f["local"]], f["values"]) if field["id"] == 3 and "developer_index" not in field)
            self.assertEqual(hr, [98])
        read = self.query("read", "--id", a["id"])
        self.assertEqual(read["sha256"], new["sha256"])
        self.assertIn("record.subsecond_ns", [s["name"] for s in read["streams"]])
        (self.root / new["path"]).write_bytes(b"keep")
        partial = self.query("export", *day, "--out", "rides", code=3)
        self.assertEqual(partial["status"], "partial")
        self.assertEqual((self.root / new["path"]).read_bytes(), b"keep")

    def test_ambiguous_id_and_truncated_archive(self):
        archive(self.zip, duplicate=True)
        self.assertEqual(self.query("list", code=3)["status"], "error")
        with zipfile.ZipFile(self.zip, "w") as z:
            z.writestr("export.xml", "<HealthData><Record/>")
        self.assertEqual(self.query("list", code=3)["status"], "error")
