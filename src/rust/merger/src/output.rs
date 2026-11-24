//! Preserve the Bryton wire payload for every unselected field, including private data.
use crate::{
    Error, Result,
    data_source::{decode, number},
    native::{self as native_fields, NativeValue},
    policy::SelectedRide,
    ride::{Ride, heart},
};
use fit_inspector::wire::{self, Field, Record};
use rustyfit::proto::Value;
use std::io;
pub const APP_ID: [u8; 16] = [
    0xeb, 0x4a, 0xe3, 0x09, 0x26, 0x0a, 0x49, 0xe4, 0xba, 0xb0, 0x9c, 0x09, 0x13, 0xb5, 0xac, 0x4d,
];

struct Cell {
    def: Field,
    data: Vec<u8>,
}
fn native(id: u8, base: u8, data: Vec<u8>) -> Cell {
    Cell {
        def: Field {
            id,
            base: Some(base),
            size: data.len() as u8,
            developer: None,
        },
        data,
    }
}
fn int_bytes(x: u16, big: bool) -> Vec<u8> {
    if big {
        x.to_be_bytes().to_vec()
    } else {
        x.to_le_bytes().to_vec()
    }
}
fn set(cells: &mut Vec<Cell>, new: Cell) -> io::Result<()> {
    if let Some(f) = cells
        .iter_mut()
        .find(|x| x.def.id == new.def.id && x.def.developer == new.def.developer)
    {
        if f.def.base != new.def.base {
            return Err(wire::invalid("unsupported width/type of selected field"));
        }
        *f = new;
    } else if new.def.developer.is_none() {
        let i = cells
            .iter()
            .position(|f| f.def.developer.is_some())
            .unwrap_or(cells.len());
        cells.insert(i, new);
    } else {
        cells.push(new);
    }
    Ok(())
}
fn definition(local: usize, n: u16, big: bool, cells: &[Cell]) -> io::Result<Vec<u8>> {
    let k = cells
        .iter()
        .take_while(|x| x.def.developer.is_none())
        .count();
    let d = cells.len() - k;
    let count =
        |v| u8::try_from(v).map_err(|_| wire::invalid("too many fields in a FIT definition"));
    let mut buf = vec![
        0x40 | (if d > 0 { 0x20 } else { 0 }) | local as u8,
        0,
        u8::from(big),
    ];
    buf.extend(int_bytes(n, big));
    buf.push(count(k)?);
    for f in &cells[..k] {
        buf.extend([f.def.id, f.def.size, f.def.base.unwrap()]);
    }
    if d > 0 {
        buf.push(count(d)?);
        for f in &cells[k..] {
            buf.extend([f.def.id, f.def.size, f.def.developer.unwrap()]);
        }
    }
    Ok(buf)
}
fn metadata(body: &mut Vec<u8>, local: usize, index: u8) -> io::Result<()> {
    let mut message = |n, cells: Vec<Cell>| -> io::Result<()> {
        body.extend(definition(local, n, false, &cells)?);
        body.push(local as u8);
        for c in cells {
            body.extend(c.data);
        }
        Ok(())
    };
    message(
        207,
        vec![native(1, 13, APP_ID.to_vec()), native(3, 2, vec![index])],
    )?;
    for (id, name) in [(11, "total_calories"), (196, "metabolic_calories")] {
        let mut name = name.as_bytes().to_vec();
        name.push(0);
        message(
            206,
            vec![
                native(0, 2, vec![index]),
                native(1, 2, vec![id]),
                native(2, 2, vec![0x84]),
                native(3, 7, name),
                native(8, 7, b"kcal\0".to_vec()),
                native(14, 0x84, 18u16.to_le_bytes().to_vec()),
                native(15, 2, vec![id]),
            ],
        )?;
    }
    Ok(())
}
fn native_value(cells: &[Cell], id: u8, big: bool) -> Option<u32> {
    let c = cells
        .iter()
        .find(|c| c.def.developer.is_none() && c.def.id == id && c.data.len() <= 4)?;
    Some(wire::unsigned(&c.data, big) as u32)
}
pub struct Calories {
    pub total: Option<u16>,
    pub metabolic: Option<u16>,
    pub dev: u8,
}
impl Calories {
    pub fn active(&self) -> Option<u16> {
        self.total?.checked_sub(self.metabolic?)
    }
    pub fn read(b: &Ride, g: &Ride, require_total: bool) -> Result<Self> {
        let total = number(&g.session, 11).and_then(|v| u16::try_from(v).ok());
        if require_total && total.is_none() {
            return Err(Error::quality("selected session.total_calories missing"));
        }
        let metabolic = number(&g.session, 196).and_then(|v| u16::try_from(v).ok());
        let dev = (0..255)
            .find(|n| !b.developer_ids.contains(n))
            .ok_or_else(|| Error::data("no free developer index"))?;
        Ok(Self {
            total,
            metabolic,
            dev,
        })
    }
    fn write(&self, cells: &mut Vec<Cell>, big: bool, selected: &SelectedRide) -> io::Result<()> {
        for (id, value, enabled) in [
            (11, self.total, selected.write_total),
            (196, self.metabolic, selected.write_metabolic),
        ] {
            let v = value.unwrap_or(u16::MAX);
            if enabled {
                set(cells, native(id, 0x84, int_bytes(v, big)))?;
            }
            if !selected.write_total {
                continue;
            }
            set(
                cells,
                Cell {
                    def: Field {
                        id,
                        base: None,
                        developer: Some(self.dev),
                        size: 2,
                    },
                    data: int_bytes(v, big),
                },
            )?;
        }
        Ok(())
    }
}
pub(crate) fn matched_summary<'a>(
    b: &Ride,
    other: &'a Ride,
    lap: Option<usize>,
) -> Option<&'a rustyfit::proto::Message> {
    let m = lap.map_or(&b.session, |i| &b.laps[i]);
    let start = number(m, 2)?;
    let duration = number(m, 7)?;
    let matches = |candidate: &&rustyfit::proto::Message| {
        number(candidate, 2) == Some(start) && number(candidate, 7) == Some(duration)
    };
    if lap.is_none() {
        return matches(&&other.session).then_some(&other.session);
    }
    let mut candidates = other.laps.iter().filter(matches);
    let first = candidates.next()?;
    candidates.next().is_none().then_some(first)
}
fn summary_stream(session: bool, id: u8) -> Option<u8> {
    // Optional native lap/session summaries associated with the configured record streams.
    Some(match (session, id) {
        (true, 3 | 38) | (false, 3 | 5) => 0,
        (true, 4 | 39) | (false, 4 | 6) => 1,
        (_, 9) => 5,
        (true, 14 | 15) | (false, 13 | 14) => 6,
        (true, 124 | 125) | (false, 110 | 111) => 73,
        (true, 18 | 19 | 92 | 93) | (false, 17 | 18 | 80 | 81) => 4,
        (true, 20 | 21 | 34 | 48 | 68 | 120 | 121)
        | (false, 19 | 20 | 33 | 41 | 60 | 106 | 107) => 7,
        (true, 49 | 50 | 71) | (false, 42 | 43 | 62) => 2,
        (true, 126..=128) | (false, 112..=114) => 78,
        (true, 57 | 58 | 150) | (false, 50 | 51 | 124) => 13,
        _ => {
            let name = native_fields::NAMES
                .get(&(if session { 18 } else { 19 }, id))?
                .split_once('.')?
                .1;
            let field = name
                .strip_prefix("avg_")
                .or_else(|| name.strip_prefix("max_"))
                .unwrap_or(name);
            return native_fields::record_id(&format!("record.{field}"));
        }
    })
}
pub fn build(b: &Ride, selected: &SelectedRide) -> Result<Vec<u8>> {
    if !selected.heart_enabled
        && selected.calories.is_none()
        && selected.fields.iter().all(|f| f.primary_only)
    {
        verify(&b.bytes, selected)?;
        return Ok(b.bytes.clone());
    }
    let s = &selected.heart;
    let mut header = vec![];
    let mut body = vec![];
    let mut record = 0;
    let mut active: [Vec<u8>; 16] = Default::default();
    let mut original: [Vec<u8>; 16] = Default::default();
    let mut lap_index = 0;
    wire::walk(&b.bytes, |frame| {
        match frame.record {
            Record::Header { .. } => {
                header = frame.bytes.to_vec();
            }
            Record::Definition { local, layout } => {
                original[local] = frame.bytes.to_vec();
                if ![18, 19, 20].contains(&layout.number) {
                    active[local] = original[local].clone();
                    body.extend(frame.bytes);
                }
            }
            Record::Crc(_) => {}
            Record::Data {
                local,
                layout,
                values,
                ..
            } => {
                if ![18, 19, 20].contains(&layout.number) {
                    if active[local] != original[local] {
                        body.extend(&original[local]);
                        active[local] = original[local].clone();
                    }
                    body.extend(frame.bytes);
                    return Ok(());
                }
                let big = layout.big;
                let mut cells: Vec<_> = layout
                    .fields
                    .iter()
                    .cloned()
                    .zip(values)
                    .map(|(def, v)| Cell {
                        def,
                        data: v.to_vec(),
                    })
                    .collect();
                match layout.number {
                    20 => {
                        let p = &s.points[record];
                        record += 1;
                        if selected.heart_enabled
                            && (p.hr.is_some()
                                || cells
                                    .iter()
                                    .any(|c| c.def.developer.is_none() && c.def.id == 3))
                        {
                            set(&mut cells, native(3, 2, vec![p.hr.unwrap_or(255)]))?;
                        }
                        for field in &selected.fields {
                            if field.primary_only {
                                continue;
                            }
                            if let Some(value) = &field.values[record - 1] {
                                set(&mut cells, native(field.id, value.base, value.encoded(big)))?;
                            } else if let Some(cell) = cells
                                .iter_mut()
                                .find(|c| c.def.developer.is_none() && c.def.id == field.id)
                            {
                                let mut value =
                                    NativeValue::read(cell.def.base.unwrap(), &cell.data, big)?;
                                value.invalidate();
                                cell.data = value.encoded(big);
                            }
                        }
                    }
                    n => {
                        let session = n == 18;
                        let lap = (!session).then_some(lap_index);
                        if !session {
                            lap_index += 1;
                        }
                        cells.retain(|c| {
                            c.def.developer.is_some()
                                || summary_stream(session, c.def.id).is_none_or(|id| {
                                    selected.fields.iter().all(|f| f.id != id || f.primary_only)
                                })
                        });
                        let hr_ids = if session { [16, 17, 64] } else { [15, 16, 63] };
                        if selected.heart_enabled
                            && !selected.heart_source.is_some_and(|g| std::ptr::eq(b, g))
                        {
                            let same = selected
                                .heart_source
                                .filter(|r| b.samples.keys().eq(r.samples.keys()))
                                .and_then(|r| matched_summary(b, r, lap));
                            for id in hr_ids {
                                let valid = same.and_then(|m| number(m, id));
                                if valid.is_some() || native_value(&cells, id, big).is_some() {
                                    set(
                                        &mut cells,
                                        native(id, 2, vec![valid.map_or(255, |v| v as u8)]),
                                    )?;
                                }
                            }
                        }
                        if session {
                            if let Some(cal) = &selected.calories {
                                if selected.write_total {
                                    metadata(&mut body, local, cal.dev)?;
                                    active[local].clear();
                                }
                                cal.write(&mut cells, big, selected)?;
                            }
                        } else {
                            let same = selected
                                .calorie_source
                                .and_then(|r| matched_summary(b, r, lap));
                            for (id, enabled) in
                                [(11, selected.write_total), (155, selected.write_metabolic)]
                            {
                                if !enabled {
                                    continue;
                                }
                                let value = same.and_then(|m| number(m, id));
                                if value.is_some() || native_value(&cells, id, big).is_some() {
                                    set(
                                        &mut cells,
                                        native(
                                            id,
                                            0x84,
                                            int_bytes(value.map_or(u16::MAX, |v| v as u16), big),
                                        ),
                                    )?;
                                }
                            }
                        }
                    }
                }
                let def = definition(local, layout.number, big, &cells)?;
                if active[local] != def {
                    body.extend(&def);
                    active[local] = def;
                }
                body.push(frame.bytes[0]);
                for c in cells {
                    body.extend(c.data);
                }
            }
        }
        Ok(())
    })
    .map_err(|e| Error::data(e.to_string()))?;
    if record != s.points.len() {
        return Err(Error::data("wire and SDK record counts differ"));
    }
    if header.len() == 12 {
        header.resize(14, 0);
        header[0] = 14;
    }
    header[1] = header[1].max(0x20);
    header[4..8].copy_from_slice(
        &u32::try_from(body.len())
            .map_err(|_| Error::data("FIT body too large"))?
            .to_le_bytes(),
    );
    let n = header.len() - 2;
    let crc = wire::crc(&header[..n]);
    header[n..].copy_from_slice(&crc.to_le_bytes());
    let crc = wire::crc(&body);
    header.extend(body);
    header.extend(crc.to_le_bytes());
    verify(&header, selected)?;
    Ok(header)
}
fn verify(bytes: &[u8], selected: &SelectedRide) -> Result<()> {
    let times: Vec<_> = selected.heart.points.iter().map(|p| p.time).collect();
    let ids = selected.fields.iter().map(|f| f.id).collect();
    let records = native_fields::read_records(bytes, times.iter(), &ids)?;
    let mut points = selected.heart.points.iter().enumerate();
    let mut sessions = 0;
    for m in decode(bytes)?.messages {
        if m.num.0 == 20 {
            let (index, p) = points
                .next()
                .ok_or_else(|| Error::data("extra output record"))?;
            for f in &selected.fields {
                if records[&p.time].get(&f.id) != f.values[index].as_ref() {
                    return Err(Error::data(format!("{} read-back differs", f.name)));
                }
            }
            if number(&m, 253) != Some(p.time) || (selected.heart_enabled && heart(&m) != p.hr) {
                return Err(Error::data("output timestamp or heart rate differs"));
            }
        } else if m.num.0 == 18 {
            sessions += 1;
            let Some(cal) = &selected.calories else {
                continue;
            };
            for (id, value, enabled) in [
                (11, cal.total, selected.write_total),
                (196, cal.metabolic, selected.write_metabolic),
            ] {
                let dev = m
                    .developer_fields
                    .iter()
                    .find(|f| f.developer_data_index == cal.dev && f.num == id);
                if enabled && number(&m, id) != value.map(u32::from)
                    || selected.write_total
                        && !dev.is_some_and(
                            |f| matches!(f.value,Value::Uint16(x) if x==value.unwrap_or(u16::MAX)),
                        )
                {
                    return Err(Error::data("output calorie read-back failed"));
                }
            }
        }
    }
    if points.next().is_some() || sessions != 1 {
        return Err(Error::data("incomplete output"));
    }
    Ok(())
}
