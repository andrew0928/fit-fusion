"""Exercise the C# executable on protocol features absent from the two ride files."""

import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import unittest

from fit_fusion.offline import inspect_wire
from fit_fusion.wire import wire_frames
from test_inspect import START, fit_member, message_definition, recording


@unittest.skipUnless(shutil.which("dotnet"), ".NET SDK required for cross-language tests")
class CSharpTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.directory.cleanup)
        cls.root = Path(cls.directory.name)
        project = Path(__file__).resolve().parents[2] / "csharp/FitFusion"
        subprocess.run(["dotnet", "build", str(project), "-o", str(cls.root), "--nologo", "-v:q"],
                       check=True, capture_output=True)
        cls.dll = cls.root / "FitFusion.dll"

    def invoke(self, command, path):
        return subprocess.run(["dotnet", str(self.dll), command, str(path)],
                              capture_output=True, text=True)

    def check_valid(self, data):
        path = self.root / "control.fit"
        path.write_bytes(data)
        raw = self.invoke("raw", path)
        self.assertEqual(raw.returncode, 0, raw.stderr)
        self.assertEqual([json.loads(s) for s in raw.stdout.splitlines()], list(wire_frames(data)))
        inspection = self.invoke("inspect", path)
        self.assertEqual(inspection.returncode, 0, inspection.stderr)
        actual = json.loads(inspection.stdout)
        self.assertEqual(actual, inspect_wire(path))
        self.assertEqual(path.read_bytes(), data)
        return actual["members"]

    def test_chained_members_and_developer_index(self):
        members = self.check_valid(recording() * 2)
        self.assertEqual(members[0], members[1])
        fields = next(m for m in members[0]["messages"] if m["number"] == 20)["fields"]
        dev = next(f for f in fields if f["developer_data_index"] == 0)
        self.assertEqual((dev["number"], dev["name"], dev["sample_count"]), (3, "vendor_hr", 3))

    def test_compressed_timestamp_rollover_big_endian_arrays(self):
        body = message_definition(0, 20, [(253, 4, 0x86)]) + struct.pack("<BI", 0, START + 31)
        # Local 1 is big endian. Field 240 contains two uint16 values.
        body += bytes([0x41, 0, 1, 0, 20, 1, 240, 4, 0x84])
        body += bytes([0xa0, 0, 0, 255, 255, 0xa1, 255, 255, 255, 255])
        record = self.check_valid(fit_member(body))[0]["messages"][0]
        self.assertEqual(record["timing"]["span_s"], 2)
        self.assertEqual(record["timing"]["timestamped_count"], 3)
        self.assertEqual(record["fields"][-1]["sample_count"], 1)

    def test_corrupt_and_truncated_inputs(self):
        valid = recording()
        path = self.root / "bad.fit"
        for data in (b"", valid[:-1], bytes([14, 16]) + bytes(16), valid[:-2] + b"\0\0",
                     fit_member(b"\x08")):
            path.write_bytes(data)
            for command in ("raw", "inspect"):
                result = self.invoke(command, path)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(path.name, result.stderr)
                if command == "inspect":
                    self.assertEqual(result.stdout, "")
