import json
from pathlib import Path
from types import SimpleNamespace
import struct
import subprocess
import sys
import tempfile
import unittest

from fit_fusion.inspect import MessageStats
from fit_fusion.timing import Timing

START = 0x40000000


def definition(number, name=None, dev=None):
    return SimpleNamespace(
        def_num=number, name=name, dev_data_index=dev, is_dev=dev is not None, size=1,
        base_type=SimpleNamespace(name="uint8"),
        field=SimpleNamespace(name=name, units=None) if name else None,
    )


def sample(definition, value):
    return SimpleNamespace(def_num=definition.def_num, field_def=definition, raw_value=value)


class ReportTests(unittest.TestCase):
    def test_timing_preserves_gaps_and_clock_anomalies(self):
        timing = Timing()
        for offset in (0, 1, 1, None, 4, 3):
            timing.observe(None if offset is None else START + offset)
        report = timing.report()
        self.assertEqual(report["timestamped_count"], 5)
        self.assertEqual(report["span_s"], 4)
        self.assertEqual(report["median_interval_s"], 2)
        self.assertEqual(report["typical_hz"], 0.5)
        self.assertEqual(report["duplicate_intervals"], 1)
        self.assertEqual(report["backward_intervals"], 1)

    def test_sparse_and_colliding_fields(self):
        message = MessageStats("record", False)
        ts = definition(253, "timestamp")
        native = definition(3, "heart_rate")
        dev = definition(3, "heart_rate", dev=0)
        unknown = definition(240)
        for item in (ts, native, dev, unknown):
            message.register(item)
        for offset, value in enumerate((0, None, 5)):
            fields = [sample(ts, START + offset), sample(native, value),
                      sample(dev, None), sample(unknown, (None, 0))]
            message.observe(SimpleNamespace(fields=fields))
        _, native_hr, dev_hr, mystery = message.report(20)["fields"]
        self.assertEqual(native_hr["timing"]["typical_hz"], 0.5)
        self.assertAlmostEqual(native_hr["coverage_pct"], 200 / 3)
        self.assertEqual(dev_hr["developer_data_index"], 0)
        self.assertEqual(dev_hr["sample_count"], 0)
        self.assertIsNone(dev_hr["timing"]["typical_hz"])
        self.assertTrue(mystery["unknown"])
        self.assertEqual(mystery["sample_count"], 3)

    def test_compressed_and_missing_timestamps(self):
        message = MessageStats("record", False)
        dev = definition(253, "vendor_time", dev=0)
        compressed = SimpleNamespace(def_num=253, field_def=None, raw_value=START)
        for fields in ([sample(dev, START + 40)], [compressed], []):
            message.observe(SimpleNamespace(fields=fields))
        report = message.report(20)
        self.assertEqual(report["message_count"], 3)
        self.assertEqual(report["timing"]["timestamped_count"], 1)
        self.assertIsNone(report["timing"]["min_interval_s"])


def fit_member(body):
    header = struct.pack("<BBHI4s", 12, 0x20, 2100, len(body), b".FIT")
    crc = 0
    for byte in header + body:
        crc ^= byte
        for _ in range(8):
            crc = (crc >> 1) ^ (0xA001 if crc & 1 else 0)
    return header + body + struct.pack("<H", crc)


def message_definition(local, number, fields, developers=()):
    result = struct.pack("<BBBHB", 0x40 | (0x20 if developers else 0) | local,
                         0, 0, number, len(fields))
    result += b"".join(bytes(field) for field in fields)
    if developers:
        result += bytes([len(developers)]) + b"".join(bytes(field) for field in developers)
    return result


def recording():
    # Define a developer HR field with the same number as the native field.
    body = message_definition(0, 207, [(3, 1, 2)]) + bytes([0, 0])
    body += message_definition(1, 206, [(0, 1, 2), (1, 1, 2), (2, 1, 2), (3, 10, 7)])
    body += bytes([1, 0, 3, 2]) + b"vendor_hr\0"
    body += message_definition(2, 20, [(253, 4, 0x86), (3, 1, 2), (240, 1, 2)], [(3, 1, 0)])
    for offset, hr in ((0, 120), (1, 255), (2, 121)):
        body += struct.pack("<BIBBB", 2, START + offset, hr, 0, 80)
    body += message_definition(3, 60000, [(42, 1, 2)]) + bytes([3, 0])
    return fit_member(body)


class CLITests(unittest.TestCase):
    def run_file(self, payload):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "sample.fit"
            path.write_bytes(payload)
            result = subprocess.run([sys.executable, "-m", "fit_fusion", "inspect", str(path)],
                                    text=True, capture_output=True)
            self.assertEqual(path.read_bytes(), payload)
            return result

    def test_binary_inventory_and_chained_files(self):
        result = self.run_file(recording() * 2)
        self.assertEqual(result.returncode, 0, result.stderr)
        members = json.loads(result.stdout)["members"]
        self.assertEqual(len(members), 2)
        self.assertEqual(members[0], members[1])
        messages = {m["number"]: m for m in members[0]["messages"]}
        self.assertTrue(messages[60000]["unknown"])
        self.assertEqual(messages[60000]["fields"][0]["number"], 42)
        record = messages[20]
        self.assertEqual(record["timing"]["typical_hz"], 1)
        fields = {f["name"]: f for f in record["fields"]}
        self.assertEqual(fields["heart_rate"]["timing"]["typical_hz"], 0.5)
        self.assertTrue(fields["unknown_240"]["unknown"])
        self.assertEqual(fields["vendor_hr"]["developer_data_index"], 0)

    def test_corruption_and_empty_input(self):
        payload = recording()
        for data in (b"", b"wrong format", payload[:-1],
                     payload[:-1] + bytes([payload[-1] ^ 1])):
            with self.subTest(length=len(data)):
                result = self.run_file(data)
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, "")
                self.assertIn("sample.fit", result.stderr)


if __name__ == "__main__":
    unittest.main()
