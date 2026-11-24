"""Verify the production offline Rust merger with the independent Python FIT reader."""
import csv
import hashlib
import importlib.util
from itertools import product
import json
import os
from pathlib import Path
import re
import shlex
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


def fixture(bryton, rates, start=START, extra_sensor=42):
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
        body += struct.pack("<BIBBB", 2, start+i, hr, 0, extra_sensor)
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

    def test_agent_can_follow_merger_index_and_workflow(self):
        def cli(*args, expected=0):
            r = subprocess.run([self.cli, *args], cwd=self.dir, capture_output=True, text=True)
            self.assertEqual(r.returncode, expected, r.stdout + r.stderr)
            return r

        index = cli("--config", "absent.json", "--skill", "index").stdout
        self.assertEqual(index, cli("--skill").stdout)
        self.assertNotEqual(index, cli("--help").stdout)
        topics = re.findall(r"\x60fit-fusion --skill ([a-z]+)\x60", index)
        self.assertCountEqual(topics, ["config", "sources", "selection", "merge", "results", "publish"])
        pages = {topic: cli("--skill", topic).stdout for topic in topics}
        invalid = cli("--skill", "unknown", expected=2)
        self.assertFalse(invalid.stdout)
        for topic in topics:
            self.assertIn(topic, invalid.stderr)
        for path in [("merge",), ("config",), ("config", "init"), ("config", "check"),
                     ("activities",), ("activities", "list")]:
            help_text = cli("--config", "absent.json", *path, "-h").stdout
            self.assertIn(" ".join(path), help_text)
            self.assertNotEqual(help_text, cli("--help").stdout)

        def examples(topic, replacements=None):
            lines = re.findall(r"^```sh\n(.*?)\n```$", pages[topic], re.S | re.M)
            self.assertTrue(lines, topic)
            for block in lines:
                for line in block.splitlines():
                    cmd, *args = shlex.split(line)
                    self.assertEqual(cmd, "fit-fusion")
                    args = [(replacements or {}).get(arg, arg) for arg in args]
                    yield json.loads(cli(*args).stdout)

        created, check = examples("config")
        self.assertEqual(created["code"], "created")
        self.assertEqual(check["primary_source"], "bike_meter")
        self.assertEqual(check["streams"], {
            "record.heart_rate": ["bike_meter", "health_band"],
            "session.total_calories": ["health_band"],
            "session.metabolic_calories": ["health_band"],
        })
        local, = re.findall(r"```json\n(.*?)\n```", pages["config"], re.S)
        (self.dir / "rides.json").write_text(local)
        fixture = ROOT / "src/rust/merger/tests/fixtures"
        for name, role, source in [("bike.fit", "bike", "rider.fit"), ("band.fit", "band", "cirqa.fit")]:
            dest = self.dir / "inbox" / role
            dest.mkdir(parents=True)
            shutil.copy(fixture / source, dest / name)
            shutil.copy(dest / name, self.dir / name)
        checked, bike, band = examples("sources")
        self.assertEqual(checked["streams"]["record.heart_rate"], ["bike_meter", "health_band"])
        self.assertEqual(bike["activities"][0]["device"]["name"], "Bryton Rider 460")
        self.assertEqual(band["activities"][0]["device"]["name"], "Garmin CIRQA")
        selection, = examples("selection")
        self.assertEqual(selection, checked)
        day, manual, auto, by_id = examples("merge", {
            "2026-11-21": "2024-01-09", "12345": band["activities"][0]["id"],
        })
        paths = [self.dir / result["artifacts"]["fit"] for result in
                 [day["rides"][0], manual, auto, by_id]]
        self.assertTrue(all(p.read_bytes() == paths[0].read_bytes() for p in paths))
        report = json.loads((self.dir / manual["artifacts"]["report"]).read_text())
        inspector, *inspect_args = shlex.split(re.search(r"```sh\n(.+?)\n```", pages["results"], re.S)[1])
        self.assertEqual(inspector, "fit-inspector")
        subprocess.run(["cargo", "build", "--release", "--locked", "--manifest-path",
                        str(ROOT / "src/rust/inspector/Cargo.toml")], cwd=ROOT, check=True)
        inventory = json.loads(subprocess.check_output(
            [Path(self.cli).with_name(inspector), *inspect_args], cwd=self.dir, text=True))
        record = next(m for m in inventory["members"][0]["messages"] if m["number"] == 20)
        self.assertEqual(record["message_count"], 6)
        checksum, = re.findall(r"```sh\n(.*?)\n```", pages["publish"], re.S)
        self.assertEqual(subprocess.check_output(shlex.split(checksum), cwd=self.dir, text=True).split()[0],
                         report["output_sha256"])
        args = ("--config", "rides.json", "merge", "--date")
        self.assertEqual(json.loads(cli(*args, "2024-01-10", "--json").stdout)["status"], "no_activities")
        shutil.copy(fixture / "rider-later.fit", self.dir / "inbox/bike/later.fit")
        partial = json.loads(cli(*args, "2024-01-09", "--json", expected=3).stdout)
        self.assertEqual(partial["status"], "partial")
        self.assertEqual([ride["code"] for ride in partial["rides"]], ["ok", "quality_review"])
        self.assertTrue(partial["rides"][0]["reused"])
        power, = re.findall(r"```json\n(.*?)\n```", pages["selection"], re.S)
        config = json.loads(local)
        config["merge"]["streams"].update(json.loads(power))
        (self.dir / "rides.json").write_text(json.dumps(config))
        updated, = examples("selection")
        self.assertEqual(updated["streams"]["record.power"], ["bike_meter", "health_band"])
        self.assertEqual(updated["streams"]["record.heart_rate"], checked["streams"]["record.heart_rate"])

    def test_help_and_version_match_cargo_and_merge_report(self):
        manifest = ROOT / "src/rust/merger/Cargo.toml"
        metadata = subprocess.check_output(["cargo", "metadata", "--format-version=1", "--no-deps",
                                            "--manifest-path", manifest], cwd=ROOT, text=True)
        package = next(p for p in json.loads(metadata)["packages"] if p["name"] == "fit-fusion")
        identity = f"fit-fusion {package['version']}"

        def run(*args):
            return subprocess.check_output([self.cli, "--config", "absent.json", *args],
                                           cwd=self.dir, text=True)

        for path in [(), ("merge",), ("config",), ("config", "init"), ("config", "check"),
                     ("activities",), ("activities", "list")]:
            self.assertTrue(run(*path, "--help").splitlines()[0].startswith(identity + " "))
        version = run("--version")
        self.assertEqual(version, run("-V"))
        self.assertEqual(version.splitlines()[0], identity)
        details = dict(line.split(maxsplit=1) for line in version.splitlines()[1:])
        self.assertTrue(details["Compiler"].startswith("rustc "))
        self.assertTrue(details["Target"])
        self.assertIn("RustyFIT", details["FIT"])
        fixture = ROOT / "src/rust/merger/tests/fixtures"
        _, report, _ = self.merge(fixture / "rider.fit", fixture / "cirqa.fit")
        self.assertEqual(details["Rules"], report["rules_version"])

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
        self.assertEqual(report["heart_rate"]["samples"], {"bike_meter":5,"health_band":1,"missing":0})
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
        self.assertEqual(check["streams"]["record.heart_rate"], ["bike_meter", "health_band"])
        fixtures = ROOT / "src/rust/merger/tests/fixtures"
        for source, files in (("bike_meter", ["rider.fit", "rider-later.fit"]), ("health_band", ["cirqa.fit"])):
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
        self.assertEqual(report["heart_rate"]["samples"], {"bike_meter": 5, "health_band": 1, "missing": 0})

    def date_setup(self):
        self.conf = self.dir / "config.json"
        self.primary = self.dir / "bikes"
        self.secondary = self.dir / "watches"
        self.primary.mkdir()
        self.secondary.mkdir()
        v = json.loads((ROOT / "docs/config/config.example.json").read_text())
        v["sources"]["bike_meter"]["directory"] = str(self.primary)
        v["sources"]["health_band"] = {"kind": "fit_directory", "device": "garmin_cirqa", "directory": str(self.secondary)}
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
        self.configured(self.primary/"renamed.fit", "--auto-match", "--out", self.dir/"auto")
        shutil.copy(scenarios/"rider-gps.fit", self.primary/"overlapping.fit")
        shared = self.configured(*day, expected=3)
        self.assertTrue(all(r["code"] == "ambiguous_match" for r in shared["rides"]))

    def test_auto_match_uses_stream_source_ids(self):
        config = self.date_setup()
        fixtures = ROOT / "src/rust/merger/tests/stream-fixtures"
        primary = fixtures / "backup.fit"
        companion = self.secondary / "head-unit.fit"
        shutil.copy(fixtures / "bike.fit", companion)
        config["sources"] = {
            "wrist": {"kind": "fit_directory", "directory": str(self.primary),
                      "device": {"manufacturer": 1, "product": 30002}},
            "head_unit": {"kind": "fit_directory", "directory": str(self.secondary),
                          "device": {"manufacturer": 267, "product": 30001}},
            "garmin": {"kind": "fit_directory", "directory": str(self.dir / "unused")},
        }
        config["merge"]["primary_source"] = "wrist"
        config["merge"]["streams"] = {"record.power": ["head_unit", "wrist"]}
        self.conf.write_text(json.dumps(config))
        matched = self.configured(primary, "--auto-match", "--out", self.dir/"auto")
        manual = self.configured(primary, companion, "--out", self.dir/"pair")
        self.assertEqual(set(matched["matched_sources"]), {"head_unit"})
        self.assertEqual(Path(matched["artifacts"]["fit"]).read_bytes(), Path(manual["artifacts"]["fit"]).read_bytes())

    def test_calories_only_override_keeps_primary_records(self):
        config = self.date_setup()
        config["merge"]["streams"] = {"session.total_calories": ["health_band"]}
        self.conf.write_text(json.dumps(config))
        scenarios = ROOT / "src/rust/merger/tests/scenarios"
        bike = self.primary / "bike.fit"
        band = self.secondary / "band.fit"
        shutil.copy(scenarios / "rider-gps.fit", bike)
        shutil.copy(scenarios / "cirqa-sensors.fit", band)
        result = self.configured("--date", "2024-01-09")["rides"][0]
        merged = Path(result["artifacts"]["fit"])
        self.assertEqual([r for n, r in rows(merged) if n == 20],
                         [r for n, r in rows(bike) if n == 20])
        def session_total(path):
            return next(r[None, 11] for n, r in rows(path) if n == 18)
        self.assertNotEqual(session_total(bike), session_total(band))
        self.assertEqual(session_total(merged), session_total(band))

    def test_auto_match_rejects_invalid_usage(self):
        self.conf = self.dir / "absent.json"
        out = self.dir / "out"
        cases = [
            (["a.fit", "--garmin", "auto"], "--auto-match"),
            (["a.fit", "--auto-match", "--auto-match"], "multiple times"),
            (["a.fit", "--auto-match", "--garmin-id", "12345"], "--garmin-id"),
            (["--auto-match"], "FIRST.fit"),
            (["a.fit", "--auto-match", "auto"], "SECOND.fit"),
            (["a.fit", "b.fit", "--auto-match"], "SECOND.fit"),
        ]
        for args, message in cases:
            with self.subTest(args=args):
                error = self.configured(*args, "--out", out, expected=2)
                self.assertEqual(error["code"], "usage")
                self.assertIn(message, error["message"])
        error = self.configured("--date", "2024-01-09", "--auto-match", expected=2)
        self.assertEqual(error["code"], "usage")
        self.assertFalse(out.exists())

    def test_activities_group_guides_the_user_to_a_working_query(self):
        args = [self.cli, "--config", self.dir / "absent.json", "activities"]
        group = subprocess.check_output(args, text=True)
        self.assertEqual(group, subprocess.check_output([*args, "--help"], text=True))
        self.assertIn("activities list --source bike_meter", group)
        missing = subprocess.run([*args, "list", "--json"], text=True, capture_output=True)
        self.assertEqual(missing.returncode, 2)
        self.assertIn("--source <ID>", json.loads(missing.stdout)["message"])
        local = ROOT / "docs/config/local-fit.example.json"
        result = subprocess.check_output([self.cli, "--config", local, "activities", "list",
                                          "--source", "bike_meter", "--json"], text=True)
        rides = json.loads(result)["activities"]
        self.assertEqual(len(rides), 2)
        self.assertTrue(all(ride["source_id"] == "bike_meter" for ride in rides))

    def test_parser_option_order_and_scoped_help(self):
        local = ROOT / "docs/config/local-fit.example.json"
        def run(*args):
            return subprocess.run([self.cli, *args], cwd=self.dir, capture_output=True, text=True)

        queries = [
            ["--config", local, "activities", "list", "--source", "bike_meter", "--json"],
            ["activities", "list", "--json", "--source", "bike_meter", "--config", local],
            ["--json", "activities", "--config", local, "list", "--source=bike_meter"],
        ]
        for args in queries:
            result = run(*args)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(json.loads(result.stdout)["activities"]), 2)
        for group in ["config", "activities"]:
            bare = run("--config", "missing.json", group)
            self.assertEqual(bare.returncode, 0, bare.stderr)
            self.assertEqual(bare.stdout, run(group, "--help").stdout)
        inline = run("merge", "missing.fit", "--help")
        self.assertEqual(inline.returncode, 0, inline.stderr)
        self.assertEqual(inline.stdout, run("merge", "--help").stdout)
        invalid = run("--config", "missing.json", "merge", "missing.fit", "--out", "out", "--json")
        self.assertEqual(invalid.returncode, 2)
        self.assertEqual(json.loads(invalid.stdout)["code"], "usage")
        self.assertFalse((self.dir / "out").exists())

    def test_parse_all_merge_modes_before_loading_settings(self):
        self.conf = self.dir / "absent.json"
        switches = (False, True)
        for count, date, auto, known, out in product(range(4), switches, switches, switches, switches):
            args = [f"{i}.fit" for i in range(count)]
            if date:
                args.extend(["--date", "2026-11-21"])
            if auto:
                args.append("--auto-match")
            if known:
                args.extend(["--garmin-id", "12345"])
            if out:
                args.extend(["--out", "out"])
            date_mode = date and count == 0 and not (auto or known or out)
            pair = count == 2 and not (auto or known)
            single = count == 1 and auto != known
            file_mode = not date and out and (pair or single)
            with self.subTest(args=args):
                result = self.configured(*args, expected=2)
                self.assertEqual(result["code"], "invalid_config" if date_mode or file_mode else "usage")
        self.assertFalse((self.dir / "out").exists())

    def test_date_config_changes_fit_and_keeps_original(self):
        v = self.date_setup()
        scenarios = ROOT / "src/rust/merger/tests/scenarios"
        main = self.primary/"original.fit"
        watch = self.secondary/"watch.fit"
        shutil.copy(scenarios/"rider-gps.fit", main)
        shutil.copy(scenarios/"cirqa-sensors.fit", watch)
        day = ("--date", "2024-01-09")
        initial = self.configured(*day)["rides"][0]
        v["merge"]["streams"] = {"record.position_lat":["bike_meter"], "record.position_long":["bike_meter"],
                                 "record.cadence": ["health_band","bike_meter"]}
        self.conf.write_text(json.dumps(v))
        new = self.configured(*day)["rides"][0]
        self.assertNotEqual(new["revision_id"], initial["revision_id"])
        report = json.loads(Path(new["artifacts"]["report"]).read_text())
        self.assertEqual(report["stream_sources"]["record.cadence"][0]["source"], "health_band")
        records = list(rows(Path(new["artifacts"]["fit"])))
        for n, fields in records:
            if n == 20:
                self.assertEqual(fields[None,13], 22)
                self.assertIn((None,3), fields)
            if n == 18:
                for key in [57,11,196]:
                    self.assertIn((None,key), fields)
        points = [f for n,f in records if n == 20]
        self.assertEqual(len(points),6)
        self.assertEqual((points[2][None,0],points[2][None,1]),(2**31-1,2**31-1))
        self.assertEqual(points[0][None,4],0)
        self.assertEqual(main.read_bytes(),(scenarios/"rider-gps.fit").read_bytes())
        v["merge"]["streams"]={"record.cadence":["bike_meter"]}
        self.conf.write_text(json.dumps(v))
        watch.unlink()
        self.secondary.rmdir()
        only=self.configured(*day)["rides"][0]
        self.assertEqual(list(json.loads(Path(only["artifacts"]["report"]).read_text())["inputs"]),["bike_meter"])

    def test_m09_parquet_revisions_and_conflicts(self):
        config = self.date_setup()
        fixtures = ROOT / "src/rust/merger/tests/fixtures"
        shutil.copy(fixtures / "rider.fit", self.primary / "ride.fit")
        shutil.copy(fixtures / "cirqa.fit", self.secondary / "watch.fit")
        args = ("--date", "2024-01-09")
        plain = self.configured(*args)["rides"][0]
        initial = self.configured(*args, "--parquet")["rides"][0]
        self.assertNotIn("parquet", plain["artifacts"])
        self.assertNotEqual(plain["revision_id"], initial["revision_id"])
        self.assertEqual(Path(plain["artifacts"]["fit"]).read_bytes(),
                         Path(initial["artifacts"]["fit"]).read_bytes())
        archive = Path(initial["artifacts"]["parquet"])
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        report = json.loads(Path(initial["artifacts"]["report"]).read_text())
        self.assertEqual(report["parquet"], {"schema_version": 2, "sha256": digest})
        self.assertTrue(self.configured(*args, "--parquet")["rides"][0]["reused"])
        config["merge"]["streams"]["record.temperature"] = ["health_band", "bike_meter"]
        self.conf.write_text(json.dumps(config))
        new = self.configured(*args, "--parquet")["rides"][0]
        self.assertNotEqual(new["revision_id"], initial["revision_id"])
        self.assertEqual(hashlib.sha256(archive.read_bytes()).hexdigest(), digest)
        target = Path(new["artifacts"]["parquet"])
        target.write_bytes(b"damaged")
        self.assertEqual(self.configured(*args, "--parquet", expected=3)["rides"][0]["code"], "output_conflict")
        self.assertEqual(target.read_bytes(), b"damaged")
        target.unlink()
        self.assertEqual(self.configured(*args, "--parquet", expected=3)["rides"][0]["code"], "output_conflict")
        self.assertFalse(target.exists())

    def verify_archive(self, db, path, sources):
        db.read_parquet(path).create_view("audit")
        manifest = json.loads(db.execute(
            "SELECT decode(value) FROM parquet_kv_metadata(?) WHERE decode(key) = 'fit_fusion'", [path]
        ).fetchone()[0])
        self.assertEqual(manifest["schema_version"], 2)
        for source, original in sources.items():
            frames = db.execute("SELECT frame_offset, frame_bytes FROM audit WHERE row_kind='frame' "
                                "AND source_id=? ORDER BY frame_offset", [source]).fetchall()
            restored = bytearray()
            for offset, data in frames:
                self.assertEqual(offset, len(restored))
                restored.extend(data)
            self.assertEqual(restored, original.read_bytes())
            self.assertEqual(hashlib.sha256(restored).hexdigest(), manifest["input_sha256"][source])
        self.assertEqual(set(manifest["input_sha256"]), set(sources))
        unmatched = db.sql("""SELECT count(*) FROM
            (SELECT * FROM audit WHERE row_kind='selected' AND value_int IS NOT NULL) s
            ANTI JOIN (SELECT * FROM audit WHERE row_kind='sample' AND developer_index IS NULL) r
            USING (source_id, frame_offset, message_id, field_id, value_int)""").fetchone()[0]
        self.assertEqual(unmatched, 0)
        return manifest

    @unittest.skipUnless(importlib.util.find_spec("duckdb"), "independent Parquet read-back requires duckdb")
    def test_m09_parquet_independent_readback(self):
        import duckdb
        config = self.date_setup()
        self.b.write_bytes(fixture(True, [110, 111, 255, 114, 114, 115]))
        self.g.write_bytes(fixture(False, [109, 110, 113, 113, 114, 115]))
        config["merge"]["streams"]["record.speed"] = ["bike_meter", "health_band"]
        for key in config["merge"]["streams"].values():
            key[:] = ["cycle_computer" if s == "bike_meter" else "watch" for s in key]
        config["sources"]["cycle_computer"] = config["sources"].pop("bike_meter")
        config["sources"]["watch"] = config["sources"].pop("health_band")
        config["merge"]["primary_source"] = "cycle_computer"
        self.conf.write_text(json.dumps(config))
        result = self.configured(self.g, self.b, "--out", self.dir / "audit", "--parquet")
        path = result["artifacts"]["parquet"]
        with duckdb.connect() as db:
            self.verify_archive(db, path, {"cycle_computer": self.b, "watch": self.g})
            heart = db.sql("SELECT source_id, value_int FROM audit WHERE row_kind='selected' "
                           "AND field_name='record.heart_rate' ORDER BY timestamp_fit_s").fetchall()
            self.assertEqual(heart, list(zip(["cycle_computer"] * 2 + ["watch"] + ["cycle_computer"] * 3,
                                            [110, 111, 113, 114, 114, 115])))
            self.assertEqual(db.sql("SELECT DISTINCT value_int FROM audit WHERE row_kind='sample' "
                                    "AND message_id=20 AND field_id=240").fetchall(), [(42,)])
            developer_values = db.sql("SELECT value_json FROM audit WHERE row_kind='sample' AND message_id=18 "
                                      "AND field_id=11 AND developer_index=0").fetchall()
            self.assertEqual([json.loads(value) for (value,) in developer_values], ["4d", "4d"])
            self.assertGreater(db.sql("SELECT count(*) FROM audit WHERE row_kind='frame' AND frame_kind='data' "
                                      "AND timestamp_fit_s IS NULL").fetchone()[0], 0)
            zeros = db.sql("SELECT count(*) FROM audit WHERE row_kind='selected' AND field_name='record.cadence' "
                           "AND value_int=0 AND source_id='cycle_computer'").fetchone()[0]
            self.assertEqual(zeros, 6)
            missing = db.sql("SELECT count(*) FROM audit WHERE row_kind='selected' AND field_name='record.speed' "
                             "AND value_int IS NULL AND frame_offset IS NULL AND source_id IS NULL").fetchone()[0]
            self.assertEqual(missing, 6)
            self.assertEqual(db.sql("SELECT value_int FROM audit WHERE row_kind='derived' "
                                    "AND field_name='session.active_calories'").fetchall(), [(1,)])

        config["merge"]["streams"] = {"record.cadence": ["cycle_computer"]}
        self.conf.write_text(json.dumps(config))
        result = self.configured(self.b, self.g, "--out", self.dir / "primary_only", "--parquet")
        with duckdb.connect() as db:
            db.read_parquet(result["artifacts"]["parquet"]).create_view("audit")
            self.assertEqual(db.sql("SELECT DISTINCT source_id FROM audit WHERE row_kind='frame' "
                                    "ORDER BY source_id").fetchall(), [("cycle_computer",), ("watch",)])
            self.assertEqual(db.sql("SELECT DISTINCT source_id FROM audit WHERE row_kind='selected' AND value_int IS NOT NULL").fetchall(),
                             [("cycle_computer",)])
            self.assertEqual(db.sql("SELECT count(*) FROM audit WHERE row_kind='selected' AND field_name='record.heart_rate' AND value_int IS NOT NULL").fetchone()[0], 5)

    @unittest.skipUnless(importlib.util.find_spec("duckdb") and (ROOT/"data/index/rides.csv").is_file(),
                         "real Parquet read-back requires duckdb and private ride fixtures")
    def test_m09_six_parquet_rides(self):
        import duckdb
        config = self.date_setup()
        for source, directory in [("bike_meter", "data/sources/bryton/fit"), ("health_band", "data/sources/garmin")]:
            config["sources"][source]["directory"] = str(ROOT / directory)
        self.conf.write_text(json.dumps(config))
        with (ROOT / "data/index/rides.csv").open() as f:
            rides = list(csv.DictReader(f))
        for ride in rides:
            date = ride["ride_date"]
            with self.subTest(date=date), duckdb.connect() as db:
                result = self.configured("--date", date, "--parquet")["rides"][0]
                sources = {"bike_meter": ROOT / ride["bryton_fit"], "health_band": ROOT / ride["garmin_fit"]}
                manifest = self.verify_archive(db, result["artifacts"]["parquet"], sources)
                fit = Path(result["artifacts"]["fit"])
                self.assertEqual(hashlib.sha256(fit.read_bytes()).hexdigest(), manifest["merged_fit_sha256"])
                self.assertEqual(db.sql("SELECT value_int FROM audit WHERE row_kind='derived' "
                                        "AND field_name='session.active_calories'").fetchone()[0],
                                 int(ride["garmin_active_kcal"]))
                if date == "2026-11-21":
                    ranges = [t for t in manifest["heart_rate"]["triggers"] if t["reason"] == "stale"]
                    self.assertEqual([t["seconds"] for t in ranges], [68, 86])
                    for t, (bpm, low, high) in zip(ranges, [(102, 102, 112), (148, 138, 148)]):
                        rates = db.execute("""SELECT row_kind, source_id, min(value_int), max(value_int), count(*)
                            FROM audit WHERE field_name='record.heart_rate' AND timestamp_fit_s BETWEEN ? AND ?
                            GROUP BY row_kind, source_id ORDER BY row_kind, source_id""",
                                           [t["start_fit_seconds"], t["end_fit_seconds"]]).fetchall()
                        self.assertEqual(rates, [("sample", "bike_meter", bpm, bpm, t["seconds"]),
                                                 ("sample", "health_band", low, high, t["seconds"]),
                                                 ("selected", "health_band", low, high, t["seconds"])])
                    self.assertEqual(db.sql("SELECT source_id, count(*) FROM audit WHERE row_kind='selected' "
                                            "AND field_name='record.heart_rate' GROUP BY source_id").fetchall(),
                                     [("health_band", 14149)])

    def test_new_devices_and_native_streams_config_v1(self):
        config = self.date_setup()
        fixtures = ROOT / "src/rust/merger/tests/stream-fixtures"
        bike, backup = self.primary / "bike.fit", self.secondary / "backup.fit"
        shutil.copy(fixtures / "bike.fit", bike)
        shutil.copy(fixtures / "backup.fit", backup)
        config["sources"] = {
            "bike": {"kind": "fit_directory", "directory": str(self.primary),
                     "device": {"manufacturer": 267, "product": 30001}},
            "backup": {"kind": "fit_directory", "directory": str(self.secondary),
                       "device": {"manufacturer": 1, "product": 30002}},
        }
        config["merge"]["primary_source"] = "bike"
        del config["merge"]["streams"]
        self.conf.write_text(json.dumps(config))
        args = ("--date", "2024-01-09")
        original = self.configured(*args)["rides"][0]
        self.assertEqual(Path(original["artifacts"]["fit"]).read_bytes(), bike.read_bytes())
        config["merge"]["streams"] = {}
        self.conf.write_text(json.dumps(config))
        same = self.configured(*args)["rides"][0]
        self.assertEqual(same["revision_id"], original["revision_id"])
        self.assertTrue(same["reused"])
        config["merge"]["streams"] = {"record.power": ["bike", "backup"],
                                     "record.left_right_balance": ["backup"],
                                     "record.left_power_phase": ["backup"]}
        self.conf.write_text(json.dumps(config))
        fused = self.configured(*args)["rides"][0]
        out = Path(fused["artifacts"]["fit"])
        records = [r for n, r in rows(out) if n == 20]
        self.assertEqual([r.get((None, 7)) for r in records], [0, 120, 170, 250, 180, 65535])
        self.assertEqual([r[None, 30] for r in records], [52] * 6)
        self.assertEqual([r[None, 69] for r in records], [[32, 64]] * 6)
        self.assertEqual(self.configured(backup, bike, "--out", self.dir/"manual")["revision_id"], fused["revision_id"])
        self.assertEqual((self.dir/"manual/merged.fit").read_bytes(), out.read_bytes())
        report = json.loads(Path(fused["artifacts"]["report"]).read_text())
        self.assertEqual([r["source"] for r in report["stream_sources"]["record.power"]],
                         ["bike", "backup", "bike", None])
        self.assertTrue(self.configured(*args)["rides"][0]["reused"])

    def test_unprofiled_native_field_by_config_name(self):
        config = self.date_setup()
        config["merge"]["streams"] = {"record.extra_sensor": ["health_band", "bike_meter"]}
        config["merge"]["field_ids"] = {"record.extra_sensor": 240}
        self.conf.write_text(json.dumps(config))
        self.b.write_bytes(fixture(True, [110] * 6))
        self.g.write_bytes(fixture(False, [109] * 6, extra_sensor=56))
        result = self.configured(self.b, self.g, "--out", self.dir / "private_field")
        fit = Path(result["artifacts"]["fit"])
        self.assertEqual([r[None, 240] for n, r in rows(fit) if n == 20], [56] * 6)
        report = json.loads(Path(result["artifacts"]["report"]).read_text())
        self.assertEqual(report["stream_sources"]["record.extra_sensor"][0]["source"], "health_band")

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
        config["sources"]["bike_meter"]["directory"]=str(ROOT/"data/sources/bryton/fit")
        config["sources"]["health_band"]["directory"]=str(ROOT/"data/sources/garmin")
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
                self.assertEqual(report["heart_rate"]["samples"]["health_band"],count)
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
                self.assertEqual(hashlib.sha256(b.read_bytes()).hexdigest(),report["inputs"]["bike_meter"]["sha256"])
                self.assertEqual(hashlib.sha256(g.read_bytes()).hexdigest(),report["inputs"]["health_band"]["sha256"])
