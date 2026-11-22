"""Lossless, profile-independent inspection of the FIT wire format."""

import math
import struct
from io import BytesIO

# FIT base type numbers 0 through 16, in protocol order.
FORMATS = ("B", "b", "B", "h", "H", "i", "I", "s", "f", "d", "B", "H", "I", "B", "q", "Q", "Q")


def crc16(data):
    crc = 0
    for byte in data:
        crc ^= byte
        for _ in range(8):
            crc = (crc >> 1) ^ (0xA001 if crc & 1 else 0)
    return crc


def take(stream, count):
    data = stream.read(count)
    if len(data) != count:
        raise ValueError("truncated FIT frame")
    return data


def wire_value(raw, base_type, endian):
    if base_type is None or (base_type & 31) >= len(FORMATS):
        return raw.hex()
    fmt = FORMATS[base_type & 31]
    if fmt == "s":
        return raw.partition(b"\0")[0].decode("utf-8", errors="replace")
    if len(raw) % struct.calcsize(fmt):
        return raw.hex()
    values = [value for value, in struct.iter_unpack(endian + fmt, raw)]
    # Preserve non-finite values in the JSON view. The frame hex is exact.
    if fmt in ("f", "d"):
        values = [value if math.isfinite(value) else str(value) for value in values]
    return values[0] if len(values) == 1 else values


def body_frames(body, base_offset):
    stream = BytesIO(body)
    definitions = {}
    while stream.tell() < len(body):
        start = stream.tell()
        header = take(stream, 1)[0]
        compressed = bool(header & 0x80)
        local = (header >> 5) & 3 if compressed else header & 15
        if not compressed and header & 0x40:
            _, arch = take(stream, 2)
            if arch not in (0, 1):
                raise ValueError(f"invalid FIT architecture {arch}")
            endian = ">" if arch else "<"
            number, count = struct.unpack(endian + "HB", take(stream, 3))
            fields = []
            for _ in range(count):
                field_id, size, base = take(stream, 3)
                fields.append({"id": field_id, "size": size, "base": base})
            if header & 0x20:
                for _ in range(take(stream, 1)[0]):
                    field_id, size, index = take(stream, 3)
                    fields.append({"id": field_id, "size": size, "developer_index": index})
            definitions[local] = number, endian, fields
            frame = {"kind": "definition", "endian": endian, "fields": fields}
        else:
            if local not in definitions:
                raise ValueError(f"undefined FIT local message {local} at {base_offset + start}")
            number, endian, fields = definitions[local]
            values = [wire_value(take(stream, f["size"]), f.get("base"), endian) for f in fields]
            frame = {"kind": "data", "values": values}
            if compressed:
                frame["timestamp_offset"] = header & 31
        yield {"offset": base_offset + start, "local": local, "global_id": number,
               "hex": body[start:stream.tell()].hex(), **frame}


def wire_frames(data):
    if not data:
        raise ValueError("empty FIT file")
    stream = BytesIO(data)
    while stream.tell() < len(data):
        start = stream.tell()
        fixed = take(stream, 12)
        length, protocol, profile, size, magic = struct.unpack("<BBHI4s", fixed)
        if magic != b".FIT" or (length != 12 and length < 14):
            raise ValueError(f"invalid FIT header at {start}")
        header = fixed + take(stream, length - 12)
        header_crc = int.from_bytes(header[-2:], "little") if length >= 14 else None
        if header_crc and crc16(header[:-2]) != header_crc:
            raise ValueError(f"FIT header CRC mismatch at {start}")
        body = take(stream, size)
        footer = take(stream, 2)
        checksum = int.from_bytes(footer, "little")
        if crc16(header + body) != checksum:
            raise ValueError(f"FIT file CRC mismatch at {start}")
        yield {"offset": start, "kind": "header", "hex": header.hex(), "profile": profile,
               "protocol": protocol, "body_size": size, "header_crc": header_crc}
        yield from body_frames(body, start + length)
        yield {"offset": start + length + size, "kind": "crc", "hex": footer.hex(), "crc": checksum}
