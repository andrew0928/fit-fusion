"""Acceptance through all three executables, using only generated FIT files."""
import os
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import unittest

TOOLS = Path(__file__).resolve().parents[1]
sys.path[:0] = [str(TOOLS), str(TOOLS.parent/"python"), str(TOOLS.parent/"python/tests")]
from verify_roundtrip import build_commands, compare_file
from test_inspect import START, fit_member, message_definition, recording


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

    def test_rust_help_and_skills(self):
        for args in (["--help"],["--skill","index"],["--skill","raw"],["--skill","inspect"],["--skill","roundtrip"]):
            r=self.invoke("rust",*args)
            self.assertEqual(r.returncode,0)
            self.assertIn("fit-inspector",r.stdout)
        self.assertEqual(self.invoke("rust","--skill","unknown").returncode,2)
