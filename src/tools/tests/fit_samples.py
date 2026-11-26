"""Synthetic FIT construction and independent wire read-back for CLI tests."""
from pathlib import Path
import struct
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path[:0] = [str(ROOT / "src/python"), str(ROOT / "src/python/tests")]
from fit_fusion.wire import wire_frames
from test_inspect import fit_member, message_definition

START = 0x40000000


def fixture(bryton, rates, start=START, extra_sensor=42, metabolic=1):
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
    body += struct.pack("<BIIBIHHB", 3, start+n-1, start, 2, n*1000, 6 if bryton else 2, metabolic, 77)
    return fit_member(body)


def rows(path):
    layouts = {}
    for f in wire_frames(path.read_bytes()):
        if f["kind"] == "definition":
            layouts[f["local"]] = f["fields"]
        if f["kind"] == "data":
            yield f["global_id"], {(d.get("developer_index"), d["id"]): v for d, v in zip(layouts[f["local"]], f["values"])}
