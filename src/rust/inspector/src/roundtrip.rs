use crate::wire::{self, Record, invalid, width};
use embedded_io_adapters::std::FromStd;
use fit_inspector::{inventory::Developers, wire::timestamp};
use rustyfit::{Decoder, Encoder};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Result, Write};
use std::path::Path;
use tempfile::NamedTempFile;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Sample {
    developer: Option<u8>,
    id: u8,
    base: Option<u8>,
    bytes: Vec<u8>,
}
#[derive(Debug, PartialEq, Eq)]
struct Row {
    number: u16,
    timestamp: Option<i64>,
    fields: Vec<Sample>,
}
struct Member {
    version: (u8, u16),
    rows: Vec<Row>,
    string_widths: BTreeMap<(usize, Option<u8>, u8), Vec<usize>>,
    messages: BTreeSet<u16>,
    definitions: BTreeSet<(u16, Option<u8>, u8, Option<u8>)>,
}

// Compare decoded scalar bit patterns (including invalids and NaN), independent
// of local definition numbers and endianness. Keep embedded string separators.
fn semantic(input: &[u8]) -> Result<Vec<Member>> {
    let mut members = Vec::<Member>::new();
    let mut dev = Developers::default();
    let mut clock = None;
    wire::walk(input, |frame| {
        match frame.record {
            Record::Header {
                protocol, profile, ..
            } => {
                members.push(Member {
                    version: (protocol, profile),
                    rows: Vec::new(),
                    string_widths: BTreeMap::new(),
                    messages: Default::default(),
                    definitions: Default::default(),
                });
                dev = Developers::default();
                clock = None;
            }
            Record::Definition { layout, .. } => {
                let member = members.last_mut().unwrap();
                member.messages.insert(layout.number);
                for f in &layout.fields {
                    member
                        .definitions
                        .insert((layout.number, f.developer, f.id, dev.base(f)));
                }
            }
            Record::Data {
                layout,
                values,
                offset,
                ..
            } => {
                let t = timestamp(layout, &values, offset, &mut clock);
                let mut fields = Vec::new();
                let member = members.last_mut().unwrap();
                let mut normalized_timestamp = false;
                for (f, raw) in layout.fields.iter().zip(&values) {
                    let base = dev.base(f);
                    // Compressed headers can be re-encoded as an explicit timestamp.
                    if f.developer.is_none()
                        && f.id == 253
                        && base == Some(0x86)
                        && raw.len() == 4
                        && !normalized_timestamp
                        && Some(wire::unsigned(raw, layout.big) as i64) == t
                    {
                        normalized_timestamp = true;
                        continue;
                    }
                    let mut bytes = raw.to_vec();
                    if base.map(|b| b & 31) == Some(7) {
                        member
                            .string_widths
                            .entry((member.rows.len(), f.developer, f.id))
                            .or_default()
                            .push(bytes.len());
                        bytes.truncate(bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1));
                    } else if layout.big
                        && let Some(w) = base.and_then(width)
                        && bytes.len().is_multiple_of(w)
                    {
                        for n in bytes.chunks_exact_mut(w) {
                            n.reverse();
                        }
                    }
                    fields.push(Sample {
                        developer: f.developer,
                        id: f.id,
                        base,
                        bytes,
                    });
                }
                if offset.is_some() && t.is_none() {
                    return Err(invalid("compressed timestamp has no preceding clock"));
                }
                fields.sort_by_key(|s| (s.developer, s.id));
                member.rows.push(Row {
                    number: layout.number,
                    timestamp: t,
                    fields,
                });
                dev.observe(layout, &values);
            }
            Record::Crc(_) => {}
        }
        Ok(())
    })?;
    Ok(members)
}

pub fn export(input: &[u8], output: &Path) -> Result<Value> {
    let before = semantic(input)?;
    let mut bytes = input;
    let mut writer = Cursor::new(Vec::new());
    let mut decoder = Decoder::builder().expand_components(false).build();
    while let Some(mut fit) = decoder
        .decode(&mut bytes)
        .map_err(|e| invalid(format!("RustyFIT decode: {e:?}")))?
    {
        Encoder::new()
            .encode(&mut FromStd::new(&mut writer), &mut fit)
            .map_err(|e| invalid(format!("RustyFIT encode: {e:?}")))?;
    }
    let encoded = writer.into_inner();
    let after = semantic(&encoded)?;
    if before.len() != after.len() {
        return Err(invalid("roundtrip member count changed"));
    }
    let mut rows = 0;
    let mut widths = 0;
    for (mi, (a, b)) in before.iter().zip(&after).enumerate() {
        if a.version != b.version || a.rows.len() != b.rows.len() {
            return Err(invalid(format!(
                "member {mi}: version or message count changed"
            )));
        }
        if !a.messages.is_subset(&b.messages) || !a.definitions.is_subset(&b.definitions) {
            return Err(invalid(format!(
                "member {mi}: original field definition missing or changed"
            )));
        }
        for (i, (left, right)) in a.rows.iter().zip(&b.rows).enumerate() {
            if left != right {
                let field = left
                    .fields
                    .iter()
                    .zip(&right.fields)
                    .find(|(a, b)| a != b)
                    .map(|(f, _)| (f.developer, f.id));
                return Err(invalid(format!(
                    "member {mi} row {i} message {}: data differs (field {field:?})",
                    left.number
                )));
            }
        }
        for (key, original) in &a.string_widths {
            let new = b
                .string_widths
                .get(key)
                .ok_or_else(|| invalid("string field missing on read-back"))?;
            widths += original
                .iter()
                .zip(new)
                .filter(|(left, right)| left != right)
                .count();
        }
        rows += a.rows.len();
    }
    let dir = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(&encoded)?;
    tmp.persist_noclobber(output)?;
    Ok(
        json!({"backend":"RustyFIT 0.11.2","output":output,"members":before.len(),"messages":rows,
        "semantic_equal":true,"byte_equal":input==encoded,"changed_string_widths":widths,
        "input_sha256":hex::encode(Sha256::digest(input)),"output_sha256":hex::encode(Sha256::digest(&encoded))}),
    )
}
