//! Lossless FIT frames and queryable field selections in one Parquet file.
use crate::{
    config::Settings,
    data_source::number,
    native::{NAMES, NativeValue},
    policy::{MatchedRide, SelectedRide},
    ride::Ride,
};
use arrow_array::{
    RecordBatch,
    builder::{ArrayBuilder, BinaryBuilder, Int64Builder, StringBuilder},
};
use arrow_schema::{DataType, Field, Schema, SchemaRef};
use fit_inspector::wire::{self, Record};
use parquet::{
    arrow::ArrowWriter,
    basic::Compression,
    file::{metadata::KeyValue, properties::WriterProperties},
};
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs::File, io, sync::Arc};

pub(super) const SCHEMA_VERSION: u8 = 2;

#[derive(Clone, Copy, Default)]
struct Row<'a> {
    kind: &'a str,
    source: Option<&'a str>,
    offset: Option<i64>,
    frame: Option<&'a str>,
    bytes: Option<&'a [u8]>,
    message: Option<u16>,
    field: Option<u8>,
    developer: Option<u8>,
    time: Option<u32>,
    name: Option<&'a str>,
    value: Option<i64>,
    other: Option<&'a str>,
}

#[derive(Default)]
struct Columns {
    kind: StringBuilder,
    source: StringBuilder,
    offset: Int64Builder,
    frame: StringBuilder,
    bytes: BinaryBuilder,
    message: Int64Builder,
    field: Int64Builder,
    developer: Int64Builder,
    time: Int64Builder,
    name: StringBuilder,
    value: Int64Builder,
    other: StringBuilder,
}
struct Table<'a> {
    schema: SchemaRef,
    writer: ArrowWriter<&'a mut File>,
    columns: Columns,
}
impl<'a> Table<'a> {
    fn new(file: &'a mut File, manifest: Value) -> io::Result<Self> {
        use DataType::{Binary, Int64, Utf8};
        let fields = [
            ("row_kind", Utf8),
            ("source_id", Utf8),
            ("frame_offset", Int64),
            ("frame_kind", Utf8),
            ("frame_bytes", Binary),
            ("message_id", Int64),
            ("field_id", Int64),
            ("developer_index", Int64),
            ("timestamp_fit_s", Int64),
            ("field_name", Utf8),
            ("value_int", Int64),
            ("value_json", Utf8),
        ]
        .map(|(name, ty)| Field::new(name, ty, name != "row_kind"));
        let schema = Arc::new(Schema::new(fields.to_vec()));
        let properties = WriterProperties::builder()
            .set_key_value_metadata(Some(vec![KeyValue::new(
                "fit_fusion".into(),
                manifest.to_string(),
            )]))
            .set_compression(Compression::SNAPPY)
            .set_max_row_group_row_count(Some(65536))
            .build();
        let writer = ArrowWriter::try_new(file, schema.clone(), Some(properties))
            .map_err(io::Error::other)?;
        Ok(Self {
            schema,
            writer,
            columns: Columns::default(),
        })
    }
    fn push(&mut self, r: Row<'_>) -> io::Result<()> {
        let c = &mut self.columns;
        c.kind.append_value(r.kind);
        c.source.append_option(r.source);
        c.offset.append_option(r.offset);
        c.frame.append_option(r.frame);
        c.bytes.append_option(r.bytes);
        c.message.append_option(r.message.map(i64::from));
        c.field.append_option(r.field.map(i64::from));
        c.developer.append_option(r.developer.map(i64::from));
        c.time.append_option(r.time.map(i64::from));
        c.name.append_option(r.name);
        c.value.append_option(r.value);
        c.other.append_option(r.other);
        if c.kind.len() == 8192 {
            self.flush()?;
        }
        Ok(())
    }
    fn flush(&mut self) -> io::Result<()> {
        let c = &mut self.columns;
        if c.kind.is_empty() {
            return Ok(());
        }
        let batch = RecordBatch::try_new(
            self.schema.clone(),
            vec![
                Arc::new(c.kind.finish()),
                Arc::new(c.source.finish()),
                Arc::new(c.offset.finish()),
                Arc::new(c.frame.finish()),
                Arc::new(c.bytes.finish()),
                Arc::new(c.message.finish()),
                Arc::new(c.field.finish()),
                Arc::new(c.developer.finish()),
                Arc::new(c.time.finish()),
                Arc::new(c.name.finish()),
                Arc::new(c.value.finish()),
                Arc::new(c.other.finish()),
            ],
        )
        .map_err(io::Error::other)?;
        self.writer.write(&batch).map_err(io::Error::other)
    }
}

#[derive(Default)]
struct Offsets {
    records: BTreeMap<u32, i64>,
    session: Option<i64>,
    laps: Vec<i64>,
}
fn raw_source(
    table: &mut Table<'_>,
    id: &str,
    ride: &Ride,
    primary: bool,
    settings: &Settings,
) -> io::Result<Offsets> {
    let overrides: BTreeMap<_, _> = settings
        .record_fields()
        .map(|(name, id)| (id, name))
        .collect();
    let mut offsets = Offsets::default();
    let mut record_times = ride.records.keys();
    wire::walk(&ride.bytes, |frame| {
        let offset = frame.at as i64;
        let mut row = Row {
            kind: "frame",
            source: Some(id),
            offset: Some(offset),
            bytes: Some(frame.bytes),
            ..Row::default()
        };
        row.frame = Some(match frame.record {
            Record::Header { .. } => "header",
            Record::Crc(_) => "crc",
            Record::Definition { layout, .. } => {
                row.message = Some(layout.number);
                "definition"
            }
            Record::Data {
                layout, ref values, ..
            } => {
                row.message = Some(layout.number);
                row.time = layout.fields.iter().zip(values).find_map(|(field, bytes)| {
                    (field.developer.is_none()
                        && field.id == 253
                        && field.base == Some(0x86)
                        && bytes.len() == 4)
                        .then(|| wire::unsigned(bytes, layout.big) as u32)
                        .filter(|&t| t != u32::MAX)
                });
                match layout.number {
                    20 => {
                        let time = *record_times
                            .next()
                            .ok_or_else(|| wire::invalid("record index exceeds source timeline"))?;
                        if row.time.is_some_and(|t| t != time) {
                            return Err(wire::invalid(
                                "wire timestamp differs from source timeline",
                            ));
                        }
                        row.time = Some(time);
                        offsets.records.insert(time, offset);
                    }
                    18 => offsets.session = Some(offset),
                    19 => offsets.laps.push(offset),
                    _ => {}
                }
                for (field, bytes) in layout.fields.iter().zip(values) {
                    let value = wire::decoded(bytes, field.base, layout.big);
                    let integer = value.as_i64();
                    let other = integer.is_none().then(|| value.to_string());
                    let name = field
                        .developer
                        .is_none()
                        .then(|| NAMES.get(&(layout.number, field.id)))
                        .flatten();
                    let sample = Row {
                        kind: "sample",
                        bytes: None,
                        field: Some(field.id),
                        developer: field.developer,
                        name: name.map(String::as_str).or_else(|| {
                            (layout.number == 20 && field.developer.is_none())
                                .then(|| overrides.get(&field.id).copied())
                                .flatten()
                        }),
                        value: integer,
                        other: other.as_deref(),
                        ..row
                    };
                    table.push(sample)?;
                    if primary
                        && layout.number == 20
                        && (field.developer.is_some()
                            || (field.id != 253 && !overrides.contains_key(&field.id)))
                    {
                        let valid = match field.base {
                            Some(base) => NativeValue::read(base, bytes, layout.big)
                                .map(|v| v.valid())
                                .unwrap_or(true),
                            None => true,
                        };
                        let selected = if valid {
                            sample
                        } else {
                            Row {
                                value: None,
                                other: None,
                                source: None,
                                offset: None,
                                ..sample
                            }
                        };
                        table.push(Row {
                            kind: "selected",
                            ..selected
                        })?;
                    }
                }
                "data"
            }
        });
        table.push(row)
    })?;
    if record_times.next().is_some() {
        return Err(wire::invalid("source records missing from frame archive"));
    }
    Ok(offsets)
}

fn write_selected(
    table: &mut Table<'_>,
    pair: &MatchedRide,
    selected: &SelectedRide<'_>,
    settings: &Settings,
    offsets: &BTreeMap<&str, Offsets>,
) -> io::Result<()> {
    let mut sample =
        |time: u32, field: u8, name: &str, value: Option<Value>, source: Option<&str>| {
            let offset = source
                .map(|id| {
                    offsets[id]
                        .records
                        .get(&time)
                        .copied()
                        .ok_or_else(|| wire::invalid("selected value missing its source frame"))
                })
                .transpose()?;
            let integer = value.as_ref().and_then(Value::as_i64);
            let other = value.filter(|_| integer.is_none()).map(|v| v.to_string());
            table.push(Row {
                kind: "selected",
                source,
                offset,
                message: Some(20),
                field: Some(field),
                time: Some(time),
                name: Some(name),
                value: integer,
                other: other.as_deref(),
                ..Row::default()
            })
        };
    if selected.heart_enabled {
        let sources: Vec<_> = settings
            .sources_for("record.heart_rate")
            .map(|s| s.id())
            .collect();
        for p in &selected.heart.points {
            let source = match p.source {
                "bryton" => sources.first(),
                "garmin" => sources.get(1),
                _ => None,
            }
            .copied();
            sample(
                p.time,
                3,
                "record.heart_rate",
                p.hr.map(Value::from),
                source,
            )?;
        }
    }
    for f in &selected.fields {
        let mut ranges = f.ranges.iter().peekable();
        for (&time, value) in pair.primary().records.keys().zip(&f.values) {
            if ranges.peek().is_some_and(|r| time > r.end_fit_seconds) {
                ranges.next();
            }
            let r = ranges
                .peek()
                .ok_or_else(|| wire::invalid("selected field missing provenance range"))?;
            sample(
                time,
                f.id,
                &f.name,
                value.as_ref().map(NativeValue::decoded),
                r.source.as_deref(),
            )?;
        }
    }
    let source_id = |ride: &Ride| {
        pair.rides
            .iter()
            .find_map(|(id, r)| std::ptr::eq(ride, r).then_some(id.as_str()))
    };
    let primary = pair.primary();
    for (lap_index, primary_message) in std::iter::once((None, &primary.session))
        .chain(primary.laps.iter().enumerate().map(|(i, m)| (Some(i), m)))
    {
        let session = lap_index.is_none();
        let message = if session { 18 } else { 19 };
        let mut summary =
            |kind, field: Option<u8>, name: &str, source: Option<&Ride>, value: Option<i64>| {
                let source = source.filter(|_| value.is_some());
                let id = source.and_then(source_id);
                let offset = match (id, source, lap_index) {
                    (Some(id), _, None) => offsets[id].session,
                    (Some(id), Some(ride), Some(i)) if std::ptr::eq(ride, primary) => {
                        offsets[id].laps.get(i).copied()
                    }
                    (Some(id), Some(ride), Some(_)) => {
                        crate::output::matched_summary(primary, ride, lap_index)
                            .and_then(|m| ride.laps.iter().position(|l| std::ptr::eq(l, m)))
                            .and_then(|i| offsets[id].laps.get(i).copied())
                    }
                    _ => None,
                };
                if value.is_some() && offset.is_none() {
                    return Err(wire::invalid("selected summary missing its source frame"));
                }
                table.push(Row {
                    kind,
                    source: id,
                    offset,
                    message: Some(message),
                    field,
                    time: number(primary_message, 253),
                    name: Some(name),
                    value,
                    ..Row::default()
                })
            };
        if selected.heart_enabled {
            let source = selected.heart_source.filter(|r| {
                std::ptr::eq(primary, *r) || primary.samples.keys().eq(r.samples.keys())
            });
            let m = source.and_then(|r| {
                if std::ptr::eq(r, primary) {
                    Some(primary_message)
                } else {
                    crate::output::matched_summary(primary, r, lap_index)
                }
            });
            for field in if session { [16, 17, 64] } else { [15, 16, 63] } {
                let name = &NAMES[&(message, field)];
                summary(
                    "selected",
                    Some(field),
                    name,
                    source,
                    m.and_then(|m| number(m, field)).map(i64::from),
                )?;
            }
        }
        if let Some(cal) = &selected.calories {
            let source = selected.calorie_source;
            let m = source.and_then(|r| {
                if session {
                    Some(&r.session)
                } else {
                    crate::output::matched_summary(primary, r, lap_index)
                }
            });
            for (field, enabled) in [
                (11, selected.write_total),
                (if session { 196 } else { 155 }, selected.write_metabolic),
            ] {
                if enabled {
                    let name = &NAMES[&(message, field)];
                    summary(
                        "selected",
                        Some(field),
                        name,
                        source,
                        m.and_then(|m| number(m, field)).map(i64::from),
                    )?;
                }
            }
            if session {
                summary(
                    "derived",
                    None,
                    "session.active_calories",
                    source,
                    cal.active().map(i64::from),
                )?;
            }
        }
    }
    Ok(())
}

pub(super) fn write(
    file: &mut File,
    pair: &MatchedRide,
    selected: &SelectedRide<'_>,
    settings: &Settings,
    report: &Value,
) -> io::Result<()> {
    let hashes: BTreeMap<_, _> = pair
        .rides
        .iter()
        .map(|(id, ride)| (id, &ride.hash))
        .collect();
    let manifest = json!({
        "schema_version": SCHEMA_VERSION,
        "input_sha256": hashes,
        "primary_source": pair.primary_id,
        "rules_version": report["rules_version"],
        "selection": settings.selection(),
        "unlisted_streams": "preserve_primary",
        "merged_fit_sha256": report["output_sha256"],
        "heart_rate": report["heart_rate"],
    });
    let mut table = Table::new(file, manifest)?;
    let mut offsets = BTreeMap::new();
    for (id, ride) in &pair.rides {
        offsets.insert(
            id.as_str(),
            raw_source(&mut table, id, ride, id == &pair.primary_id, settings)?,
        );
    }
    write_selected(&mut table, pair, selected, settings, &offsets)?;
    table.flush()?;
    table.writer.close().map_err(io::Error::other)?;
    Ok(())
}
