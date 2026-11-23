import struct
import unittest

from fit_fusion.wire import wire_frames
from test_inspect import fit_member, message_definition, recording


class WireTests(unittest.TestCase):
    def test_raw_fields_and_lossless_chained_members(self):
        member = recording()
        frames = list(wire_frames(member * 2))
        self.assertEqual(b"".join(bytes.fromhex(f["hex"]) for f in frames), member * 2)
        records = [f for f in frames if f.get("global_id") == 20 and f["kind"] == "data"]
        self.assertEqual(records[1]["values"][1:], [255, 0, "50"])
        self.assertEqual(frames[-1]["offset"], 2 * len(member) - 2)

    def test_big_endian_and_compressed_header(self):
        schema = bytes([0x41, 0, 1]) + struct.pack(">HB", 60000, 1) + bytes([2, 2, 0x84])
        data = fit_member(schema + bytes([0xa2, 0x10, 0x20]))
        record = list(wire_frames(data))[2]
        self.assertEqual(record["timestamp_offset"], 2)
        self.assertEqual(record["values"], [0x1020])

    def test_bad_crc_length_and_undefined_messages(self):
        valid = recording()
        for data in [b"", valid[:-1], valid[:-1] + bytes([valid[-1] ^ 1]),
                     fit_member(b"\x06"),
                     fit_member(message_definition(0, 20, [(3, 1, 2)]) + b"\0")]:
            with self.subTest(length=len(data)), self.assertRaises(ValueError):
                list(wire_frames(data))
