// FIT wire values stay in native scale. BigInt is retained through JSON serialization.
export const json = (value) =>
  JSON.stringify(value, (_, v) => typeof v === "bigint" ? JSON.rawJSON(v.toString()) : v);
const widths = [1, 1, 1, 2, 2, 4, 4, 1, 4, 8, 1, 2, 4, 1, 8, 8, 8];
const methods = [
  "UInt8",
  "Int8",
  "UInt8",
  "Int16",
  "UInt16",
  "Int32",
  "UInt32",
  null,
  "Float",
  "Double",
  "UInt8",
  "UInt16",
  "UInt32",
  "UInt8",
  "BigInt64",
  "BigUInt64",
  "BigUInt64",
];

function crc(bytes) {
  let result = 0;
  for (const byte of bytes) {
    result ^= byte;
    for (let bit = 0; bit < 8; bit++) result = (result >>> 1) ^ (result & 1 ? 0xa001 : 0);
  }
  return result;
}

// FIT invalid codes are per base type. An invalid element in an array stays at its index.
const invalid = [
  255,
  127,
  255,
  32767,
  65535,
  2147483647,
  4294967295,
  "",
  "nan",
  "nan",
  0,
  0,
  0,
  255,
  2n ** 63n - 1n,
  2n ** 64n - 1n,
  0n,
];
export function validValue(value, base) {
  if (Array.isArray(value)) {
    const elements = value.map((v) => validValue(v, base));
    return elements.every((v) => v === null) ? null : elements;
  }
  return value === invalid[base & 31] ? null : value;
}

function method(base, endian) {
  return methods[base] + (widths[base] === 1 ? "" : endian === ">" ? "BE" : "LE");
}

export function decodeValue(bytes, base, endian) {
  const type = base === undefined ? -1 : base & 31;
  if (!widths[type] || bytes.length % widths[type]) return bytes.toString("hex");
  if (type === 7) {
    const zero = bytes.indexOf(0);
    return bytes.toString("utf8", 0, zero < 0 ? bytes.length : zero);
  }
  const result = [];
  for (let i = 0; i < bytes.length; i += widths[type]) {
    const n = bytes["read" + method(type, endian)](i);
    result.push(Number.isNaN(n) ? "nan" : n === Infinity ? "inf" : n === -Infinity ? "-inf" : n);
  }
  return result.length === 1 ? result[0] : result;
}

export function* frames(bytes) {
  if (!bytes.length) throw Error("empty FIT");
  let pos = 0, limit = bytes.length;
  function take(n) {
    if (pos + n > limit) throw Error("truncated FIT at " + pos);
    const part = bytes.subarray(pos, pos + n);
    pos += n;
    return part;
  }
  while (pos < bytes.length) {
    const offset = pos, head = take(12), length = head[0], body_size = head.readUInt32LE(4);
    if ((length !== 12 && length < 14) || !head.subarray(8, 12).equals(Buffer.from(".FIT"))) {
      throw Error("invalid FIT header");
    }
    take(length - 12);
    const header = bytes.subarray(offset, pos);
    const header_crc = length === 12 ? null : header.readUInt16LE(length - 2);
    if (header_crc && crc(header.subarray(0, -2)) !== header_crc) throw Error("header CRC mismatch");
    limit = pos + body_size;
    if (limit + 2 > bytes.length || crc(bytes.subarray(offset, limit)) !== bytes.readUInt16LE(limit)) {
      throw Error("truncated member or file CRC mismatch");
    }
    yield {
      kind: "header",
      offset,
      protocol: head[1],
      profile: head.readUInt16LE(2),
      body_size,
      header_crc,
      hex: header.toString("hex"),
    };
    const layouts = new Map();
    while (pos < limit) {
      const offset = pos, flag = take(1)[0], compressed = Boolean(flag & 128);
      const local = compressed ? (flag >> 5) & 3 : flag & 15;
      let frame;
      if (!compressed && flag & 64) {
        const def = take(5);
        if (def[1] > 1) throw Error("invalid architecture at " + offset);
        const endian = def[1] ? ">" : "<";
        const global_id = def[1] ? def.readUInt16BE(2) : def.readUInt16LE(2);
        const fields = [];
        for (let i = 0; i < def[4]; i++) {
          const [id, size, base] = take(3);
          fields.push({ id, size, base });
        }
        if (flag & 32) {
          const count = take(1)[0];
          for (let i = 0; i < count; i++) {
            const [id, size, developer_index] = take(3);
            fields.push({ id, size, developer_index });
          }
        }
        frame = { kind: "definition", global_id, fields, endian };
        layouts.set(local, frame);
      } else {
        const def = layouts.get(local);
        if (!def) throw Error("undefined local message at " + offset);
        const values = def.fields.map((f) => decodeValue(take(f.size), f.base, def.endian));
        frame = { kind: "data", global_id: def.global_id, values };
        if (compressed) frame.timestamp_offset = flag & 31;
      }
      yield { ...frame, offset, local, hex: bytes.toString("hex", offset, pos) };
    }
    const footer = pos;
    limit = bytes.length;
    const checksum = take(2);
    yield { kind: "crc", offset: footer, crc: checksum.readUInt16LE(), hex: checksum.toString("hex") };
  }
}

function encodeField(value, original, base, endian) {
  const type = base === undefined ? -1 : base & 31;
  if (!widths[type] || original.length % widths[type]) return Buffer.from(value, "hex");
  // Strings carry padding and potentially non-UTF8 bytes. NaN payloads also have no unique JS encoding.
  if (type === 7) return original;
  const out = Buffer.alloc(original.length);
  for (const [i, v] of (Array.isArray(value) ? value : [value]).entries()) {
    const at = i * widths[type];
    if (v === "nan") original.copy(out, at, at, at + widths[type]);
    else out["write" + method(type, endian)](v === "inf" ? Infinity : v === "-inf" ? -Infinity : v, at);
  }
  return out;
}

export function encode(input) {
  const output = [];
  let header, body, layouts, developers, members = 0, messages = 0;
  for (const f of frames(input)) {
    if (f.kind === "header") {
      members++;
      header = Buffer.from(f.hex, "hex");
      body = [];
      layouts = new Map();
      developers = new Map();
    } else if (f.kind === "definition") {
      layouts.set(f.local, f);
      const native = f.fields.filter((x) => x.base !== undefined);
      const dev = f.fields.slice(native.length);
      const h = Buffer.from([
        64 | f.local | (dev.length ? 32 : 0),
        0,
        Number(f.endian === ">"),
        0,
        0,
        native.length,
      ]);
      h[f.endian === ">" ? "writeUInt16BE" : "writeUInt16LE"](f.global_id, 3);
      body.push(h);
      for (const field of native) body.push(Buffer.from([field.id, field.size, field.base]));
      if (dev.length) body.push(Buffer.from([dev.length]));
      for (const field of dev) body.push(Buffer.from([field.id, field.size, field.developer_index]));
    } else if (f.kind === "data") {
      messages++;
      const def = layouts.get(f.local), payload = Buffer.from(f.hex, "hex"), native = {};
      body.push(
        Buffer.from([f.timestamp_offset === undefined ? f.local : 128 | f.local << 5 | f.timestamp_offset]),
      );
      let at = 1;
      for (const [i, field] of def.fields.entries()) {
        const raw = payload.subarray(at, at + field.size);
        at += field.size;
        const base = field.base ?? developers.get([field.developer_index, field.id].join());
        const value = field.base === undefined ? decodeValue(raw, base, def.endian) : f.values[i];
        if (field.base !== undefined) native[field.id] = value;
        const encoded = encodeField(value, raw, base, def.endian);
        if (!encoded.equals(raw)) throw Error("field re-encoding differs at " + f.offset);
        body.push(encoded);
      }
      if (f.global_id === 206) developers.set([native[0], native[1]].join(), native[2]);
    } else {
      const data = Buffer.concat(body);
      if (header.length === 12) header = Buffer.concat([header, Buffer.alloc(2)]);
      header[0] = header.length;
      header.writeUInt32LE(data.length, 4);
      header.writeUInt16LE(crc(header.subarray(0, -2)), header.length - 2);
      const tail = Buffer.alloc(2);
      tail.writeUInt16LE(crc(data));
      output.push(header, data, tail);
    }
  }
  return { bytes: Buffer.concat(output), members, messages };
}
