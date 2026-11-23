"""Serialize parsed wire fields using their FIT definitions; publish only a verified new file."""
from itertools import zip_longest
import hashlib
import os
import struct
from tempfile import NamedTemporaryFile

from .wire import FORMATS, crc16, wire_frames, wire_value


def encode_field(value, original, base, endian):
    kind = base & 31 if base is not None else None
    if kind is None or kind >= len(FORMATS):
        return bytes.fromhex(value)
    fmt = FORMATS[kind]
    if fmt == "s":
        # The public wire view shows the first string. Keep later strings, padding
        # and non-UTF8 bytes as separate wire data for lossless serialization.
        first, separator, tail = original.partition(b"\0")
        encoded = value.encode("utf-8")
        if encoded != first and value == first.decode("utf-8", "replace"):
            encoded = first  # Preserve malformed UTF-8 that the JSON view replaces.
        return encoded + separator + tail
    width = struct.calcsize(fmt)
    if len(original) % width:
        return bytes.fromhex(value)
    values = value if isinstance(value, list) else [value]
    encoded = bytearray()
    for index, item in enumerate(values):
        if fmt in ("f", "d") and item == "nan":
            # JSON represents all NaN payloads with the same string.
            bits = original[index * width:(index + 1) * width]
            encoded.extend(bits)
        else:
            encoded.extend(struct.pack(endian + fmt, float(item) if fmt in ("f", "d") else item))
    return encoded


def encode_member(data):
    output = bytearray()
    members = messages = 0
    for frame in wire_frames(data):
        kind = frame["kind"]
        if kind == "header":
            header = bytes.fromhex(frame["hex"])
            layouts, developers = {}, {}
            body = bytearray()
            members += 1
        elif kind == "definition":
            local, number = frame["local"], frame["global_id"]
            endian, fields = frame["endian"], frame["fields"]
            layouts[local] = (endian, fields)
            native = [f for f in fields if "base" in f]
            dev = [f for f in fields if "developer_index" in f]
            body.extend(struct.pack(endian + "BBBHB", 0x40 | (0x20 if dev else 0) | local,
                                    0, int(endian == ">"), number, len(native)))
            for f in native:
                body.extend((f["id"], f["size"], f["base"]))
            if dev:
                body.append(len(dev))
                for f in dev:
                    body.extend((f["id"], f["size"], f["developer_index"]))
        elif kind == "data":
            local = frame["local"]
            endian, fields = layouts[local]
            compressed = frame.get("timestamp_offset")
            body.append(local if compressed is None else 0x80 | local << 5 | compressed)
            payload = bytes.fromhex(frame["hex"])
            offset, native = 1, {}
            for f, value in zip(fields, frame["values"]):
                raw = payload[offset:offset + f["size"]]
                offset += len(raw)
                base = f.get("base")
                if "developer_index" in f:
                    base = developers.get((f["developer_index"], f["id"]))
                    value = wire_value(raw, base, endian)
                else:
                    native[f["id"]] = value
                field = encode_field(value, raw, base, endian)
                if field != raw:
                    raise ValueError(f"field {f['id']} changed at frame {frame['offset']}")
                body.extend(field)
            if frame["global_id"] == 206 and all(isinstance(native.get(i), int) for i in (0, 1, 2)):
                developers[native[0], native[1]] = native[2]
            messages += 1
        else:
            extension = header[12:-2]
            h = struct.pack("<BBHI4s", 14 + len(extension), header[1],
                            int.from_bytes(header[2:4], "little"), len(body), b".FIT") + extension
            output.extend(h + struct.pack("<H", crc16(h)) + body + struct.pack("<H", crc16(body)))
    return output, members, messages


def frame_content(f):
    if f is None:
        return None
    if f["kind"] == "header":
        return ("header", f["protocol"], f["profile"])
    if f["kind"] == "crc":
        return ("crc",)
    if f["kind"] == "definition":
        return ("definition", f["local"], f["global_id"], f["endian"], f["fields"])
    return ("data", f["local"], f["global_id"], f.get("timestamp_offset"), f["hex"][2:])


def roundtrip(source, dest):
    data = source.read_bytes()
    result, members, messages = encode_member(data)
    for ordinal, (left, right) in enumerate(zip_longest(wire_frames(data), wire_frames(result))):
        if frame_content(left) != frame_content(right):
            raise ValueError(f"read-back differs at frame {ordinal}")
    with NamedTemporaryFile(dir=dest.parent) as temp:
        temp.write(result)
        temp.flush()
        os.link(temp.name, dest)  # Atomic create, including when dest is an alias of the input.
    return {"backend": "Python wire", "output": str(dest), "members": members, "messages": messages,
            "semantic_equal": True, "byte_equal": data == result,
            "input_sha256": hashlib.sha256(data).hexdigest(), "output_sha256": hashlib.sha256(result).hexdigest()}
