//! Whole-field replacement on the primary time line. The index contains only selected fields.
use fit_inspector::{
    native::{NAMES, NativeValue},
    wire::{self, Layout, Record, invalid},
    write::{Cell, definition, native, set},
};
use std::{collections::BTreeMap, io};

const RECORD: u16 = 20;
const SESSION: u16 = 18;
type Key = (u16, u8);
type Values = BTreeMap<u8, NativeValue>;
type Range = (i64, i64);

#[derive(Default)]
struct Selection(BTreeMap<Key, String>);

impl Selection {
    fn parse(names: &[String]) -> io::Result<Self> {
        let mut fields = BTreeMap::new();
        for name in names {
            let key = NAMES
                .iter()
                .find_map(|(&key, candidate)| {
                    (candidate == name && matches!(key, (RECORD, 0..=252) | (SESSION, 11 | 196)))
                        .then_some(key)
                })
                .ok_or_else(|| invalid(format!("unsupported stream: {name}")))?;
            if fields.insert(key, name.clone()).is_some() {
                return Err(invalid(format!("duplicate stream: {name}")));
            }
        }
        Ok(Self(fields))
    }

    fn fields(&self, message: u16) -> impl Iterator<Item = (u8, &str)> {
        self.0
            .range((message, 0)..=(message, 255))
            .map(|(&(_, id), name)| (id, name.as_str()))
    }
}

#[derive(Default)]
struct Times {
    records: Option<Range>,
    sessions: Vec<Option<Range>>,
}

impl Times {
    fn observe(&mut self, layout: &Layout, values: &[&[u8]], time: Option<i64>) {
        if layout.number == RECORD
            && let Some(t) = time
        {
            let span = self.records.get_or_insert((t, t));
            span.0 = span.0.min(t);
            span.1 = span.1.max(t);
        } else if layout.number == SESSION {
            let uint32 = |id| {
                wire::native(layout, values, id)
                    .and_then(|v| v.as_i64())
                    .filter(|&v| (0..i64::from(u32::MAX)).contains(&v))
            };
            // Some recorders timestamp the session message at departure.
            // Compare the activity intervals in milliseconds, using FIT total_elapsed_time.
            let start = uint32(2).map(|seconds| seconds * 1000);
            let end = start
                .zip(uint32(7).filter(|&ms| ms > 0))
                .map(|(start, elapsed)| start + elapsed - 1)
                .or_else(|| time.map(|seconds| seconds * 1000));
            self.sessions
                .push(start.zip(end).filter(|(start, end)| start <= end));
        }
    }

    fn interval(&self, summary: bool) -> io::Result<Range> {
        if summary {
            if let [Some(span)] = self.sessions.as_slice() {
                return Ok(*span);
            }
            return Err(invalid(
                "session overwrite requires one session with a valid time range in each FIT",
            ));
        }
        self.records
            .ok_or_else(|| invalid("no timestamped record data"))
    }
}

#[derive(Default)]
struct Overlay {
    // A definition is evidence of presence even when every data value is invalid.
    missing: BTreeMap<Key, NativeValue>,
    records: BTreeMap<i64, Option<Values>>,
    session: Values,
    times: Times,
}

impl Overlay {
    fn read(bytes: &[u8], selected: &Selection) -> io::Result<Self> {
        let mut overlay = Self::default();
        let mut clock = None;
        wire::walk(bytes, |frame| {
            match frame.record {
                Record::Header { .. } => clock = None,
                Record::Definition { layout, .. } => {
                    for f in &layout.fields {
                        let key = (layout.number, f.id);
                        if let Some(base) = f.base
                            && selected.0.contains_key(&key)
                        {
                            let mut missing =
                                NativeValue::read(base, &vec![0; usize::from(f.size)], false)?;
                            missing.invalidate();
                            overlay.missing.entry(key).or_insert(missing);
                        }
                    }
                }
                Record::Data {
                    layout,
                    offset,
                    values,
                    ..
                } => {
                    let time = wire::timestamp(layout, &values, offset, &mut clock);
                    overlay.times.observe(layout, &values, time);
                    if selected.fields(layout.number).next().is_none() {
                        return Ok(());
                    }
                    let mut row = Values::new();
                    for (f, raw) in layout.fields.iter().zip(values) {
                        if let Some(base) = f.base
                            && selected.0.contains_key(&(layout.number, f.id))
                        {
                            let value = NativeValue::read(base, raw, layout.big)?;
                            if row.insert(f.id, value).is_some() {
                                return Err(invalid(format!(
                                    "ambiguous overlay field: {}",
                                    selected.0[&(layout.number, f.id)]
                                )));
                            }
                        }
                    }
                    if layout.number == RECORD {
                        let time =
                            time.ok_or_else(|| invalid("overlay record has no timestamp"))?;
                        overlay
                            .records
                            .entry(time)
                            .and_modify(|v| *v = None)
                            .or_insert(Some(row));
                    } else {
                        overlay.session = row;
                    }
                }
                Record::Crc(_) => {}
            }
            Ok(())
        })?;
        for (key, name) in &selected.0 {
            if !overlay.missing.contains_key(key) {
                return Err(invalid(format!("overlay does not contain {name}")));
            }
        }
        Ok(overlay)
    }

    fn row(&self, message: u16, time: Option<i64>) -> io::Result<Option<&Values>> {
        if message == SESSION {
            return Ok(Some(&self.session));
        }
        let t = time.ok_or_else(|| invalid("primary record has no timestamp"))?;
        match self.records.get(&t) {
            Some(None) => Err(invalid(format!(
                "ambiguous overlay records at FIT timestamp {t}"
            ))),
            Some(Some(row)) => Ok(Some(row)),
            None => Ok(None),
        }
    }
}

pub fn merge(primary: &[u8], overlay: &[u8], streams: &[String]) -> io::Result<Vec<u8>> {
    let selected = Selection::parse(streams)?;
    let overlay = Overlay::read(overlay, &selected)?;
    let mut output = Vec::with_capacity(primary.len());
    let mut active: [Vec<u8>; 16] = Default::default();
    let mut member = 0;
    let mut body = 0;
    let mut clock = None;
    let mut times = Times::default();
    wire::walk(primary, |frame| {
        match frame.record {
            Record::Header { .. } => {
                member = output.len();
                output.extend(frame.bytes);
                body = output.len();
                clock = None;
                active = Default::default();
            }
            Record::Definition { local, .. } => {
                active[local] = frame.bytes.to_vec();
                output.extend(frame.bytes);
            }
            Record::Data {
                local,
                layout,
                offset,
                values,
            } => {
                let time = wire::timestamp(layout, &values, offset, &mut clock);
                times.observe(layout, &values, time);
                if selected.fields(layout.number).next().is_none() {
                    output.extend(frame.bytes);
                    return Ok(());
                }
                let row = overlay.row(layout.number, time)?;
                let mut cells: Vec<_> = layout
                    .fields
                    .iter()
                    .cloned()
                    .zip(values)
                    .map(|(def, data)| Cell {
                        def,
                        data: data.to_vec(),
                    })
                    .collect();
                for (id, name) in selected.fields(layout.number) {
                    if layout
                        .fields
                        .iter()
                        .filter(|f| f.developer.is_none() && f.id == id)
                        .nth(1)
                        .is_some()
                    {
                        return Err(invalid(format!("ambiguous primary field: {name}")));
                    }
                    let value = row
                        .and_then(|r| r.get(&id))
                        .unwrap_or(&overlay.missing[&(layout.number, id)]);
                    set(
                        &mut cells,
                        native(id, value.base, value.encoded(layout.big)),
                    )?;
                }
                let def = definition(local, layout.number, layout.big, &cells)?;
                if active[local] != def {
                    output.extend(&def);
                    active[local] = def;
                }
                output.push(frame.bytes[0]);
                for cell in cells {
                    output.extend(cell.data);
                }
            }
            Record::Crc(_) => {
                let size = u32::try_from(output.len() - body)
                    .map_err(|_| invalid("FIT member too large"))?;
                output[member + 4..member + 8].copy_from_slice(&size.to_le_bytes());
                if body - member >= 14 {
                    let crc = wire::crc(&output[member..body - 2]);
                    output[body - 2..body].copy_from_slice(&crc.to_le_bytes());
                }
                output.extend(wire::crc(&output[member..]).to_le_bytes());
            }
        }
        Ok(())
    })?;
    for message in [RECORD, SESSION] {
        if selected.fields(message).next().is_some() {
            let (a, b) = times.interval(message == SESSION)?;
            let (c, d) = overlay.times.interval(message == SESSION)?;
            if a > d || b < c {
                return Err(invalid("primary and overlay time ranges do not overlap"));
            }
        }
    }
    wire::walk(&output, |_| Ok(()))?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn records(field: u8, samples: &[(u32, u8)]) -> Vec<u8> {
        let mut data = vec![0x40, 0, 0, 20, 0, 2, 253, 4, 0x86, field, 1, 2];
        for (time, value) in samples {
            data.push(0);
            data.extend(time.to_le_bytes());
            data.push(*value);
        }
        let mut file = vec![12, 0x20, 0, 0];
        file.extend((data.len() as u32).to_le_bytes());
        file.extend(b".FIT");
        file.extend(data);
        file.extend(wire::crc(&file).to_le_bytes());
        file
    }

    fn rates(file: &[u8]) -> Vec<u8> {
        let mut result = vec![];
        wire::walk(file, |frame| {
            if let Record::Data { layout, values, .. } = frame.record {
                let i = layout.fields.iter().position(|f| f.id == 3).unwrap();
                result.push(values[i][0]);
            }
            Ok(())
        })
        .unwrap();
        result
    }

    #[test]
    fn whole_stream_preserves_primary_clock_and_overlay_gaps() {
        let primary = records(3, &[(100, 110), (101, 110), (102, 110), (103, 110)]);
        let overlay = records(3, &[(99, 120), (100, 140), (101, 255), (103, 145)]);
        assert_eq!(
            rates(&merge(&primary, &overlay, &["record.heart_rate".into()]).unwrap()),
            [140, 255, 255, 145]
        );
    }

    #[test]
    fn absent_field_is_an_error_but_present_invalid_fields_clear_values() {
        let primary = records(3, &[(100, 110), (101, 110)]);
        let names = ["record.heart_rate".into()];
        assert!(
            merge(&primary, &records(4, &[(100, 80)]), &names)
                .unwrap_err()
                .to_string()
                .contains("record.heart_rate")
        );
        let invalid = records(3, &[(100, 255), (101, 255)]);
        assert_eq!(
            rates(&merge(&primary, &invalid, &names).unwrap()),
            [255, 255]
        );
    }

    #[test]
    fn session_interval_uses_elapsed_time_when_summary_timestamp_is_departure() {
        let layout = Layout {
            number: SESSION,
            big: false,
            fields: [2, 7]
                .map(|id| wire::Field {
                    id,
                    size: 4,
                    base: Some(0x86),
                    developer: None,
                })
                .to_vec(),
        };
        let mut times = Times::default();
        times.observe(
            &layout,
            &[&100u32.to_le_bytes(), &60500u32.to_le_bytes()],
            Some(100),
        );
        assert_eq!(times.interval(true).unwrap(), (100000, 160499));
    }

    #[test]
    fn ambiguous_or_disjoint_recordings_fail() {
        let primary = records(3, &[(100, 110), (101, 110)]);
        for other in [
            records(3, &[(100, 130), (100, 131)]),
            records(3, &[(200, 140)]),
        ] {
            assert!(merge(&primary, &other, &["record.heart_rate".into()]).is_err());
        }
    }
}
