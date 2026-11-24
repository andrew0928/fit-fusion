import { readFileSync } from "node:fs";
import { decodeValue, frames, validValue } from "./wire.mjs";

const types =
  "enum sint8 uint8 sint16 uint16 sint32 uint32 string float32 float64 uint8z uint16z uint32z byte sint64 uint64 uint64z"
    .split(" ");
const names = new Map(), labels = new Map();
const profile = readFileSync(new URL("../../python/fit_fusion/profile.tsv", import.meta.url), "utf8");
for (const line of profile.split(/\r?\n/)) {
  if (!line || line.startsWith("#")) continue;
  const [num, name, id, field, units] = line.split("\t");
  if (id) labels.set([+num, +id].join(), [field, units || null]);
  else names.set(+num, name);
}

class Timing {
  count = 0;
  first = Infinity;
  last = -Infinity;
  previous = null;
  duplicates = 0;
  backwards = 0;
  intervals = new Map();
  add(t) {
    if (t === null) return;
    this.count++;
    this.first = Math.min(this.first, t);
    this.last = Math.max(this.last, t);
    if (this.previous !== null) {
      const delta = t - this.previous;
      if (delta > 0) this.intervals.set(delta, (this.intervals.get(delta) ?? 0) + 1);
      else if (delta === 0) this.duplicates++;
      else this.backwards++;
    }
    this.previous = t;
  }
  report() {
    const intervals = [...this.intervals].sort((a, b) => a[0] - b[0]);
    const n = intervals.reduce((sum, [, v]) => sum + v, 0);
    let seen = 0, middle = 0;
    for (const [s, freq] of intervals) {
      for (const i of [Math.floor((n - 1) / 2), Math.floor(n / 2)]) {
        if (seen <= i && i < seen + freq) middle += s;
      }
      seen += freq;
    }
    const utc = (x) =>
      this.count ? new Date((631065600 + x) * 1000).toISOString().replace(".000Z", "Z") : null;
    return {
      timestamped_count: this.count,
      start_utc: utc(this.first),
      end_utc: utc(this.last),
      span_s: this.count ? this.last - this.first : null,
      min_interval_s: intervals[0]?.[0] ?? null,
      median_interval_s: n ? middle / 2 : null,
      max_interval_s: intervals.at(-1)?.[0] ?? null,
      typical_hz: n ? 2 / middle : null,
      duplicate_intervals: this.duplicates,
      backward_intervals: this.backwards,
    };
  }
}

export function inventory(file, bytes) {
  const members = [];
  let header, messages, layouts, developers, clock;
  for (const f of frames(bytes)) {
    if (f.kind === "header") {
      header = f;
      messages = new Map();
      layouts = new Map();
      developers = new Map();
      clock = null;
    } else if (f.kind === "crc") {
      members.push({
        protocol_version: [header.protocol >> 4, header.protocol & 15].join("."),
        profile_version: [Math.floor(header.profile / 100), String(header.profile % 100).padStart(2, "0")]
          .join("."),
        messages: [...messages].sort((a, b) => a[0] - b[0]).map(([number, m]) => ({
          number,
          name: names.get(number) ?? "unknown_" + number,
          unknown: !names.has(number),
          message_count: m.rows,
          timing: m.timing.report(),
          fields: [...m.fields.values()].map((s) => ({
            ...s.meta,
            encoded_sizes_bytes: [...s.sizes].sort((a, b) => a - b),
            sample_count: s.samples,
            coverage_pct: m.rows ? 100 * s.samples / m.rows : null,
            timing: s.timing.report(),
          })),
        })),
      });
    } else if (f.kind === "definition") {
      if (!messages.has(f.global_id)) {
        messages.set(f.global_id, { rows: 0, fields: new Map(), timing: new Timing() });
      }
      const m = messages.get(f.global_id);
      const bindings = f.fields.map((field) => {
        const idx = field.developer_index ?? null;
        const [name, units, devBase] = (idx === null
          ? labels.get([f.global_id, field.id].join())
          : developers.get([idx, field.id].join())) ?? [];
        const base = field.base ?? devBase ?? 13;
        const meta = {
          number: field.id,
          name: name || "unknown_" + field.id,
          base_type: types[base & 31] ?? "unknown_" + base,
          units: units || null,
          developer_data_index: idx,
          unknown: !name,
        };
        const key = JSON.stringify(meta);
        if (!m.fields.has(key)) {
          m.fields.set(key, { meta, timing: new Timing(), samples: 0, sizes: new Set() });
        }
        const stats = m.fields.get(key);
        stats.sizes.add(field.size);
        return { field, base, stats };
      });
      layouts.set(f.local, { bindings, endian: f.endian });
    } else {
      const m = messages.get(f.global_id), { bindings, endian } = layouts.get(f.local);
      const native = {};
      for (const [i, { field }] of bindings.entries()) {
        if (field.base !== undefined) native[field.id] = f.values[i];
      }
      let t = native[253];
      if (!Number.isInteger(t) || t < 0 || t >= 0xffffffff) t = null;
      if (f.timestamp_offset !== undefined && clock !== null) {
        t = clock + ((f.timestamp_offset - (clock % 32) + 32) % 32);
      }
      if (t !== null) clock = t;
      if (t < 0x10000000 || t >= 0xffffffff) t = null;
      m.rows++;
      m.timing.add(t);
      const seen = new Set();
      for (const [i, { field, base, stats }] of bindings.entries()) {
        const value = field.base === undefined
          ? decodeValue(Buffer.from(f.values[i], "hex"), base, endian)
          : f.values[i];
        if (!seen.has(stats) && validValue(value, base) !== null) {
          stats.samples++;
          stats.timing.add(t);
          seen.add(stats);
        }
      }
      if (f.global_id === 206 && [0, 1, 2].every((k) => Number.isInteger(native[k]))) {
        developers.set([native[0], native[1]].join(), [native[3], native[8], native[2]]);
      }
    }
  }
  return { file, members };
}
