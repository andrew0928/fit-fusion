"""Build the existing inspection report from the standard-library wire reader."""

from pathlib import Path

from .inspect import MessageStats
from .wire import wire_frames, wire_value


TYPES = ("enum", "sint8", "uint8", "sint16", "uint16", "sint32", "uint32",
         "string", "float32", "float64", "uint8z", "uint16z", "uint32z",
         "byte", "sint64", "uint64", "uint64z")
INVALID = (255, 127, 255, 32767, 65535, 2147483647, 4294967295, "", "nan", "nan",
           0, 0, 0, 255, 2**63 - 1, 2**64 - 1, 0)


def populated(value, base):
    if isinstance(value, list):
        return any(populated(element, base) for element in value)
    return base >= len(INVALID) or value != INVALID[base]


def profile():
    messages, fields = {}, {}
    for line in Path(__file__).with_name("profile.tsv").read_text(encoding="utf8").splitlines():
        if not line or line.startswith("#"):
            continue
        message, name, number, field, units = line.split("\t")
        if number:
            fields[int(message), int(number)] = (field, units or None)
        else:
            messages[int(message)] = name
    return messages, fields


def inspect_wire(path):
    names, labels = profile()
    members = []
    for frame in wire_frames(path.read_bytes()):
        kind = frame["kind"]
        if kind == "header":
            header = frame
            messages, layouts, developers = {}, {}, {}
            clock = None
            continue
        if kind == "crc":
            p, v = header["protocol"], header["profile"]
            members.append({"protocol_version": f"{p >> 4}.{p & 15}", "profile_version": f"{v//100}.{v%100:02}",
                            "messages": [messages[n].report(n) for n in sorted(messages)]})
            continue

        number, local = frame["global_id"], frame["local"]
        if kind == "definition":
            if number not in messages:
                messages[number] = MessageStats(names.get(number, f"unknown_{number}"), number not in names)
            bindings = []
            for field in frame["fields"]:
                idx, fid = field.get("developer_index"), field["id"]
                if idx is None:
                    name, units = labels.get((number, fid), (None, None))
                    base = field["base"]
                else:
                    name, units, base = developers.get((idx, fid), (None, None, 13))
                k = base & 31
                type_name = TYPES[k] if k < len(TYPES) else f"unknown_{base}"
                stats = messages[number].register_field(idx, fid, name or f"unknown_{fid}",
                                                        type_name, units, not name, field["size"])
                bindings.append((fid, idx, base, stats))
            layouts[local] = (frame["endian"], bindings)
            continue

        endian, bindings = layouts[local]
        values = frame["values"]
        native = {fid: value for (fid, idx, _, _), value in zip(bindings, values) if idx is None}
        timestamp = native.get(253)
        if not isinstance(timestamp, int) or not 0 <= timestamp < 0xffffffff:
            timestamp = None
        if "timestamp_offset" in frame and clock is not None:
            timestamp = clock + (frame["timestamp_offset"] - clock) % 32
        if timestamp is not None:
            clock = timestamp
        if timestamp is not None and not 0x10000000 <= timestamp < 0xffffffff:
            timestamp = None
        msg = messages[number]
        msg.rows += 1
        msg.timing.observe(timestamp)
        seen = set()
        for (_, idx, base, stats), value in zip(bindings, values):
            if idx is not None:
                value = wire_value(bytes.fromhex(value), base, endian)
            if stats not in seen and populated(value, base & 31):
                stats.samples += 1
                stats.timing.observe(timestamp)
                seen.add(stats)
        if number == 206 and all(isinstance(native.get(i), int) for i in (0, 1, 2)):
            developers[native[0], native[1]] = (native.get(3) or None, native.get(8) or None, native[2])
    return {"file": str(path), "members": members}
