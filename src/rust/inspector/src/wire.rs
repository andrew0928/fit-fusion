//! Physical FIT records. Keep field boundaries even when the profile has no name.
use serde_json::{Value, json};
use std::io::{Error, ErrorKind, Result};

pub fn invalid(reason: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, reason.into())
}

#[derive(Clone, Debug)]
pub struct Field {
    pub id: u8,
    pub size: u8,
    pub base: Option<u8>,
    pub developer: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub number: u16,
    pub big: bool,
    pub fields: Vec<Field>,
}

pub enum Record<'a> {
    Header {
        protocol: u8,
        profile: u16,
        size: u32,
    },
    Definition {
        local: usize,
        layout: &'a Layout,
    },
    Data {
        local: usize,
        layout: &'a Layout,
        offset: Option<u8>,
        values: Vec<&'a [u8]>,
    },
    Crc(u16),
}

pub struct Frame<'a> {
    pub at: usize,
    pub bytes: &'a [u8],
    pub record: Record<'a>,
}

struct Reader<'a> {
    input: &'a [u8],
    pos: usize,
    end: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        if count > self.end - self.pos {
            return Err(invalid(format!("truncated FIT at byte {}", self.pos)));
        }
        let start = self.pos;
        self.pos += count;
        Ok(&self.input[start..self.pos])
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn frame(&self, at: usize, record: Record<'a>) -> Frame<'a> {
        Frame {
            at,
            bytes: &self.input[at..self.pos],
            record,
        }
    }
}

pub fn crc(bytes: &[u8]) -> u16 {
    let mut crc = 0u16;
    for &b in bytes {
        crc ^= u16::from(b);
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xa001 } else { 0 };
        }
    }
    crc
}

pub fn walk(input: &[u8], mut visit: impl FnMut(Frame<'_>) -> Result<()>) -> Result<()> {
    if input.is_empty() {
        return Err(invalid("empty FIT file"));
    }
    let mut r = Reader {
        input,
        pos: 0,
        end: input.len(),
    };
    while r.pos < input.len() {
        let start = r.pos;
        let header = r.take(12)?;
        let len = usize::from(header[0]);
        if (len != 12 && len < 14) || &header[8..] != b".FIT" {
            return Err(invalid(format!("invalid header at byte {start}")));
        }
        r.take(len - 12)?;
        if len >= 14 {
            let expected = u16::from_le_bytes(input[r.pos - 2..r.pos].try_into().unwrap());
            if expected != 0 && crc(&input[start..r.pos - 2]) != expected {
                return Err(invalid(format!("header CRC mismatch at byte {start}")));
            }
        }
        let size = u32::from_le_bytes(header[4..8].try_into().unwrap());
        let end = r
            .pos
            .checked_add(size as usize)
            .ok_or_else(|| invalid("invalid member size"))?;
        let footer = input
            .get(end..)
            .and_then(|s| s.get(..2))
            .ok_or_else(|| invalid("truncated member"))?;
        let checksum = u16::from_le_bytes(footer.try_into().unwrap());
        if crc(&input[start..end]) != checksum {
            return Err(invalid(format!("member CRC mismatch at byte {end}")));
        }
        visit(r.frame(
            start,
            Record::Header {
                protocol: header[1],
                profile: u16::from_le_bytes([header[2], header[3]]),
                size,
            },
        ))?;
        let mut layouts: [Option<Layout>; 16] = Default::default();
        r.end = end;
        while r.pos < end {
            let at = r.pos;
            let h = r.byte()?;
            let compressed = h & 128 != 0;
            let local = usize::from(if compressed { (h >> 5) & 3 } else { h & 15 });
            if !compressed && h & 64 != 0 {
                let d = r.take(5)?;
                if d[1] > 1 {
                    return Err(invalid(format!("invalid byte order at byte {at}")));
                }
                let big = d[1] == 1;
                let number = unsigned(&d[2..4], big) as u16;
                let mut fields = Vec::new();
                for _ in 0..d[4] {
                    let f = r.take(3)?;
                    fields.push(Field {
                        id: f[0],
                        size: f[1],
                        base: Some(f[2]),
                        developer: None,
                    });
                }
                if h & 32 != 0 {
                    for _ in 0..r.byte()? {
                        let f = r.take(3)?;
                        fields.push(Field {
                            id: f[0],
                            size: f[1],
                            base: None,
                            developer: Some(f[2]),
                        });
                    }
                }
                let layout = layouts[local].insert(Layout {
                    number,
                    big,
                    fields,
                });
                visit(Frame {
                    at,
                    bytes: &input[at..r.pos],
                    record: Record::Definition { local, layout },
                })?;
            } else {
                let layout = layouts[local]
                    .as_ref()
                    .ok_or_else(|| invalid(format!("undefined local {local} at byte {at}")))?;
                let values = layout
                    .fields
                    .iter()
                    .map(|f| r.take(f.size.into()))
                    .collect::<Result<_>>()?;
                visit(Frame {
                    at,
                    bytes: &input[at..r.pos],
                    record: Record::Data {
                        local,
                        layout,
                        values,
                        offset: compressed.then_some(h & 31),
                    },
                })?;
            }
        }
        r.end = input.len();
        r.take(2)?;
        visit(r.frame(end, Record::Crc(checksum)))?;
    }
    Ok(())
}

pub const TYPES: [&str; 17] = [
    "enum", "sint8", "uint8", "sint16", "uint16", "sint32", "uint32", "string", "float32",
    "float64", "uint8z", "uint16z", "uint32z", "byte", "sint64", "uint64", "uint64z",
];
pub fn width(base: u8) -> Option<usize> {
    [1, 1, 1, 2, 2, 4, 4, 1, 4, 8, 1, 2, 4, 1, 8, 8, 8]
        .get(usize::from(base & 31))
        .copied()
}
pub fn unsigned(b: &[u8], big: bool) -> u64 {
    if big {
        b.iter().fold(0, |n, x| n << 8 | u64::from(*x))
    } else {
        b.iter().rev().fold(0, |n, x| n << 8 | u64::from(*x))
    }
}
pub fn float(v: f64) -> Value {
    if v.is_nan() {
        json!("nan")
    } else if v.is_infinite() {
        json!(if v.is_sign_positive() { "inf" } else { "-inf" })
    } else {
        json!(v)
    }
}
pub fn decoded(b: &[u8], base: Option<u8>, big: bool) -> Value {
    let Some((k, w)) = base.and_then(|t| width(t).map(|w| (t & 31, w))) else {
        return json!(hex::encode(b));
    };
    if k == 7 {
        return json!(String::from_utf8_lossy(
            b.split(|x| *x == 0).next().unwrap_or_default()
        ));
    }
    if !b.len().is_multiple_of(w) {
        return json!(hex::encode(b));
    }
    let scalar = |bytes: &[u8]| {
        let n = unsigned(bytes, big);
        match k {
            1 => json!(n as i8),
            3 => json!(n as i16),
            5 => json!(n as i32),
            14 => json!(n as i64),
            8 => float(f32::from_bits(n as u32) as f64),
            9 => float(f64::from_bits(n)),
            _ => json!(n),
        }
    };
    if b.len() == w {
        scalar(b)
    } else {
        Value::Array(b.chunks_exact(w).map(scalar).collect())
    }
}

impl Frame<'_> {
    pub fn raw(&self) -> Value {
        let mut result = match &self.record {
            Record::Header {
                protocol,
                profile,
                size,
            } => json!({"kind":"header","protocol":protocol,"profile":profile,"body_size":size,
                "header_crc":(self.bytes.len() >= 14).then(|| unsigned(&self.bytes[self.bytes.len()-2..],false))}),
            Record::Crc(crc) => json!({"kind":"crc","crc":crc}),
            Record::Definition { local, layout } => {
                let fields: Vec<_> = layout
                    .fields
                    .iter()
                    .map(|f| {
                        let mut field = json!({"id":f.id,"size":f.size});
                        if let Some(b) = f.base {
                            field["base"] = json!(b);
                        } else {
                            field["developer_index"] = json!(f.developer);
                        }
                        field
                    })
                    .collect();
                json!({"kind":"definition","local":local,"global_id":layout.number,"endian":if layout.big {">"} else {"<"},"fields":fields})
            }
            Record::Data {
                local,
                layout,
                values,
                offset,
            } => {
                let decoded: Vec<_> = values
                    .iter()
                    .zip(&layout.fields)
                    .map(|(v, f)| decoded(v, f.base, layout.big))
                    .collect();
                let mut obj =
                    json!({"kind":"data","local":local,"global_id":layout.number,"values":decoded});
                if let Some(ts) = offset {
                    obj["timestamp_offset"] = json!(ts);
                }
                obj
            }
        };
        result["offset"] = json!(self.at);
        result["hex"] = json!(hex::encode(self.bytes));
        result
    }
}

pub fn native(layout: &Layout, values: &[&[u8]], id: u8) -> Option<Value> {
    let i = layout
        .fields
        .iter()
        .rposition(|f| f.developer.is_none() && f.id == id)?;
    Some(decoded(values[i], layout.fields[i].base, layout.big))
}

pub fn timestamp(
    layout: &Layout,
    values: &[&[u8]],
    offset: Option<u8>,
    clock: &mut Option<i64>,
) -> Option<i64> {
    let full = native(layout, values, 253)
        .and_then(|v| v.as_i64())
        .filter(|v| (0..0xffffffff).contains(v));
    let t = match (*clock, offset) {
        (Some(prev), Some(s)) => Some(prev + (i64::from(s) - prev).rem_euclid(32)),
        _ => full,
    };
    if t.is_some() {
        *clock = t;
    }
    t
}
