"""Acceptance through all four executables, using only generated FIT files."""
import json
import os
import re
import shlex
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest
from xml.etree import ElementTree

TOOLS = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(TOOLS), str(TOOLS.parent/"python"), str(TOOLS.parent/"python/tests")]
from verify_roundtrip import build_commands, compare_file
from test_inspect import START, fit_member, message_definition, recording
from fit_fusion import __version__ as python_version


class RoundTripTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.build = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.build.cleanup)
        cls.commands = build_commands(Path(cls.build.name))

    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.work = Path(temp.name)
        self.source = self.work/"control.fit"

    def verify(self, data):
        self.source.write_bytes(data)
        return compare_file(self.source,self.work/"exports",self.work,self.commands)

    def invoke(self, language, *args):
        return subprocess.run([*self.commands[language], *map(str,args)], capture_output=True, text=True,
                              env={**os.environ,"PYTHONPATH":str(TOOLS.parent/"python")})

    def test_chain_developer_collision_and_exact_integers(self):
        layout = message_definition(0,60000,[(42,8,0x8f)])
        precision = fit_member(layout+struct.pack("<BQBQ",0,2**53,0,2**53+1))
        result = self.verify(recording()+precision)
        for receipt in result["exports"]:
            self.assertEqual(receipt["members"],2)
            self.assertFalse(receipt["byte_equal"])  # Source uses 12-byte headers, each encoder emits 14.

    def test_compressed_clock_and_big_endian_arrays(self):
        body = message_definition(0,20,[(253,4,0x86)]) + struct.pack("<BI",0,START+31)
        body += bytes([0x41,0,1,0,20,1,240,4,0x84,0xa0,0,0,255,255,0xa1,255,255,255,255])
        self.verify(fit_member(body))

    def test_corrupt_member_and_destination_protection(self):
        good = recording()
        dest = self.work/"existing.fit"
        dest.write_bytes(b"existing")
        self.source.write_bytes(good)
        alias = self.work/"alias.fit"
        alias.symlink_to(self.source)
        hardlink = self.work/"hardlink.fit"
        os.link(self.source,hardlink)
        for lang in self.commands:
            for path in (self.source,alias,hardlink,dest):
                with self.subTest(lang=lang,path=path.name):
                    r=self.invoke(lang,"roundtrip",self.source,"--out",path)
                    self.assertNotEqual(r.returncode,0)
                    self.assertFalse(r.stdout)
                    self.assertTrue(r.stderr)
                    self.assertEqual(self.source.read_bytes(),good)
                    self.assertEqual(dest.read_bytes(),b"existing")
            for data in (b"",good[:-1], good+good[:-1]+bytes([good[-1]^1])):
                self.source.write_bytes(data)
                for cmd in ("raw","inspect","roundtrip"):
                    out=self.work/"invalid.fit"
                    args=["--out",out] if cmd=="roundtrip" else []
                    r=self.invoke(lang,cmd,self.source,*args)
                    self.assertNotEqual(r.returncode,0,(lang,cmd))
                    if cmd!="raw": self.assertFalse(r.stdout)
                    self.assertFalse(out.exists())
            self.source.write_bytes(good)

    def test_sdk_field_loss_is_rejected(self):
        # A vendor encoded two values in the known scalar record.heart_rate field.
        # RustyFIT interprets it via the standard profile; the guard must catch the lost byte.
        self.source.write_bytes(fit_member(message_definition(0,20,[(3,2,2)])+bytes([0,92,93])))
        out=self.work/"lossy.fit"
        r=self.invoke("rust","roundtrip",self.source,"--out",out)
        self.assertNotEqual(r.returncode,0)
        self.assertIn("differs",r.stderr)
        self.assertFalse(out.exists())

    def test_untyped_developer_field_reports_sdk_gap(self):
        self.source.write_bytes(fit_member(message_definition(0,60000,[],[(3,1,0)])+bytes([0,72])))
        wire=self.invoke("rust","inspect",self.source,"--decoder","wire")
        self.assertEqual(wire.returncode,0,wire.stderr)
        sdk=self.invoke("rust","inspect",self.source)
        self.assertNotEqual(sdk.returncode,0)
        self.assertIn("field 3",sdk.stderr)
        self.assertFalse(sdk.stdout)

    def test_node_sdk_reads_vendor_fields_and_reports_encoder_limit(self):
        self.source.write_bytes(recording())
        result = self.invoke("node", "sdk-check", self.source)
        self.assertEqual(result.returncode, 0, result.stderr)
        member, = json.loads(result.stdout)["members"]
        self.assertEqual(member["sdk_messages"], member["wire_messages"])
        self.assertGreater(member["native_values_compared"], 0)
        self.assertTrue(member["native_values_equal"])
        self.assertFalse(member["encode_field_values_equal"])
        self.assertIsNotNone(member["encode_error"])

    def test_node_sdk_reads_and_reencodes_standard_record(self):
        layout = message_definition(0, 20, [(253,4,0x86),(3,1,2)])
        self.source.write_bytes(fit_member(layout + struct.pack("<BIB",0,START,120)))
        result = self.invoke("node", "sdk-check", self.source)
        self.assertEqual(result.returncode, 0, result.stderr)
        member, = json.loads(result.stdout)["members"]
        self.assertTrue(member["native_values_equal"])
        self.assertTrue(member["encode_field_values_equal"], member)
        self.assertEqual(member["native_values_compared"], 2)

    def test_follow_inspector_skill_index_and_examples(self):
        src = TOOLS.parent
        fixture = src / "rust/merger/tests/fixtures/rider.fit"
        for lang, launcher in {
            "python": "PYTHONPATH=src/python python3 -m fit_fusion",
            "csharp": "dotnet FitFusion.dll",
            "rust": "fit-inspector",
            "node": "node src/node/inspector/cli.mjs",
        }.items():
            with self.subTest(language=lang):
                topics = ["inspect", "raw", "roundtrip"] + (["sdk-check"] if lang == "node" else [])
                index = self.invoke(lang, "--skill", "index")
                self.assertEqual(index.returncode, 0, index.stderr)
                self.assertEqual(self.invoke(lang, "--skill").stdout, index.stdout)
                self.assertNotEqual(index.stdout, self.invoke(lang, "--help").stdout)
                links = re.findall(re.escape(launcher) + r" --skill ([a-z-]+)`", index.stdout)
                self.assertCountEqual(links, topics)
                dest = self.work / (lang + "-copy.fit")
                for topic in links:
                    with self.subTest(topic=topic):
                        help_page = self.invoke(lang, topic, "-h")
                        self.assertEqual(help_page.returncode, 0, help_page.stderr)
                        self.assertIn(topic, help_page.stdout)
                        self.assertIn("FILE.fit", help_page.stdout)
                        guide = self.invoke(lang, "--skill", topic)
                        self.assertEqual(guide.returncode, 0, guide.stderr)
                        example, = re.findall(r"```sh\n(.+?)\n```", guide.stdout, re.S)
                        cmd = shlex.split(example)
                        prefix = shlex.split(launcher)
                        self.assertEqual(cmd[:len(prefix)], prefix)
                        paths = {"ride.fit": str(fixture), "copy.fit": str(dest)}
                        cmd = [paths.get(word, word) for word in cmd]
                        env = os.environ.copy()
                        cwd = src.parent
                        if lang == "python":
                            key, value = cmd.pop(0).split("=", 1)
                            env[key] = value
                        elif lang == "csharp":
                            cwd = Path(self.commands[lang][1]).parent
                        elif lang == "rust":
                            env["PATH"] = str(Path(self.commands[lang][0]).parent) + os.pathsep + env["PATH"]
                        r = subprocess.run(cmd, cwd=cwd, env=env, capture_output=True, text=True)
                        self.assertEqual(r.returncode, 0, r.stderr)
                        if topic == "raw":
                            frames = [json.loads(line) for line in r.stdout.splitlines()]
                            self.assertEqual(b"".join(bytes.fromhex(f["hex"]) for f in frames), fixture.read_bytes())
                        elif topic == "roundtrip":
                            self.assertTrue(json.loads(r.stdout)["semantic_equal"])
                            self.assertTrue(dest.is_file())
                        elif topic == "inspect":
                            record = next(m for m in json.loads(r.stdout)["members"][0]["messages"] if m["number"] == 20)
                            self.assertEqual(record["message_count"], 6)
                        else:
                            self.assertIn("encode_field_values_equal", json.loads(r.stdout)["members"][0])
                invalid = self.invoke(lang, "--skill", "unknown")
                self.assertEqual(invalid.returncode, 2)
                self.assertFalse(invalid.stdout)
                for topic in topics:
                    self.assertIn(topic, invalid.stderr)

    def test_help_and_version_match_each_inspector_package(self):
        src = TOOLS.parent
        metadata = subprocess.check_output(["cargo", "metadata", "--format-version=1", "--no-deps",
                                            "--manifest-path", src / "rust/inspector/Cargo.toml"], text=True)
        rust = next(p for p in json.loads(metadata)["packages"] if p["name"] == "fit-inspector")
        versions = {
            "rust": rust["version"],
            "node": json.loads((src / "node/inspector/package.json").read_text())["version"],
            "csharp": ElementTree.parse(src / "csharp/FitFusion/FitFusion.csproj").findtext(".//Version"),
            "python": python_version,
        }
        for lang, package_version in versions.items():
            with self.subTest(language=lang):
                version = self.invoke(lang, "--version")
                alias = self.invoke(lang, "-V")
                self.assertEqual((version.returncode, alias.returncode), (0, 0))
                self.assertEqual(version.stdout, alias.stdout)
                self.assertFalse(version.stderr)
                identity, *lines = version.stdout.splitlines()
                self.assertEqual(identity.split()[:2], ["fit-inspector", package_version])
                details = dict(line.split(maxsplit=1) for line in lines)
                for field in (("Compiler", "Target", "FIT") if lang == "rust" else ("Runtime", "Platform", "FIT")):
                    self.assertTrue(details[field])
                commands = ["inspect", "raw", "roundtrip"] + (["sdk-check"] if lang == "node" else [])
                for path in [[], *([cmd] for cmd in commands)]:
                    help_page = self.invoke(lang, *path, "--help")
                    self.assertEqual(help_page.returncode, 0, help_page.stderr)
                    self.assertTrue(help_page.stdout.splitlines()[0].startswith(identity + " — "))

    def test_stream_coverage_per_member_all_backends(self):
        layout = message_definition(0, 20, [(253, 4, 0x86), (7, 2, 0x84), (3, 1, 2)])
        body = layout + b"".join(struct.pack("<BIHB", 0, START + dt, power, 255)
                                for dt, power in [(0, 0), (2, 10), (4, 40), (5, 50), (9, 65535)])
        body += message_definition(1, 20, [(253, 4, 0x86)]) + struct.pack("<BI", 1, START+15)
        empty = message_definition(0, 18, [(11, 2, 0x84)]) + bytes([0, 2, 0])
        original = fit_member(body) + fit_member(empty)
        self.source.write_bytes(original)
        for lang in self.commands:
            with self.subTest(language=lang):
                run = self.invoke(lang, "inspect", self.source, "--stream", "record.heart_rate",
                                  "--stream", "record.power", "--stream", "record.cadence")
                self.assertEqual(run.returncode, 0, run.stderr)
                first, second = json.loads(run.stdout)["members"]
                rows = {r["name"]: r for r in first["sampling"]}
                power = rows["record.power"]
                self.assertEqual((power["message_count"], power["sample_count"], power["missing_count"]), (6, 4, 2))
                self.assertAlmostEqual(power["missing_pct"], 100/3)
                self.assertEqual(rows["record.heart_rate"]["state"], "no_valid_samples")
                self.assertEqual(rows["record.cadence"]["state"], "absent")
                self.assertTrue(all(r["missing_pct"] is None and r["message_count"] == 0 for r in second["sampling"]))
                self.assertEqual(self.source.read_bytes(), original)
                self.assertEqual(self.invoke(lang, "inspect", "missing.fit", "--stream", "invalid").returncode, 2)

    def test_rust_parser_validates_flags_before_opening_files(self):
        invalid = self.invoke("rust", "inspect", "--bogus")
        self.assertEqual(invalid.returncode, 2, invalid.stderr)
        self.assertFalse(invalid.stdout)
        self.assertIn("--bogus", invalid.stderr)
        help_page = self.invoke("rust", "inspect", "missing.fit", "--help")
        self.assertEqual(help_page.returncode, 0, help_page.stderr)
        self.assertEqual(help_page.stdout, self.invoke("rust", "inspect", "--help").stdout)
        self.source.write_bytes(recording())
        reordered = self.invoke("rust", "inspect", "--decoder=wire", self.source)
        self.assertEqual(reordered.returncode, 0, reordered.stderr)
        self.assertEqual(json.loads(reordered.stdout), json.loads(self.invoke("rust", "inspect", self.source, "--decoder", "wire").stdout))
