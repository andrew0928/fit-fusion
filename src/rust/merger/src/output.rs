//! Preserve the Bryton wire payload for every unselected field, including private data.
use crate::{
    Error, Result,
    data_source::{decode, number},
    quality::Selection,
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
        if f.def.base != new.def.base || f.def.size != new.def.size {
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
    pub total: u16,
    pub metabolic: Option<u16>,
    pub dev: u8,
}
impl Calories {
    pub fn read(b: &Ride, g: &Ride) -> Result<Self> {
        let total = number(&g.session, 11)
            .and_then(|v| u16::try_from(v).ok())
            .ok_or_else(|| Error::quality("Garmin session.total_calories missing"))?;
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
    fn write(&self, cells: &mut Vec<Cell>, big: bool) -> io::Result<()> {
        for (id, v) in [(11, self.total), (196, self.metabolic.unwrap_or(u16::MAX))] {
            set(cells, native(id, 0x84, int_bytes(v, big)))?;
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
pub fn build(b: &Ride, g: &Ride, s: &Selection, cal: &Calories) -> Result<Vec<u8>> {
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
            Record::Definition { local, .. } => {
                original[local] = frame.bytes.to_vec();
                active[local] = original[local].clone();
                body.extend(frame.bytes);
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
                        if p.hr.is_some()
                            || cells
                                .iter()
                                .any(|c| c.def.developer.is_none() && c.def.id == 3)
                        {
                            set(&mut cells, native(3, 2, vec![p.hr.unwrap_or(255)]))?;
                        }
                        let lat = native_value(&cells, 0, big);
                        let lon = native_value(&cells, 1, big);
                        if lat.is_none_or(|x| x == i32::MAX as u32)
                            || lon.is_none_or(|x| x == i32::MAX as u32)
                        {
                            for (id, v) in [(0, lat), (1, lon)] {
                                if v.is_some_and(|v| v != i32::MAX as u32) {
                                    set(
                                        &mut cells,
                                        native(
                                            id,
                                            0x85,
                                            if big {
                                                i32::MAX.to_be_bytes()
                                            } else {
                                                i32::MAX.to_le_bytes()
                                            }
                                            .to_vec(),
                                        ),
                                    )?;
                                }
                            }
                        }
                    }
                    n => {
                        let session = n == 18;
                        let same_lap = if session {
                            None
                        } else {
                            let lap = &b.laps[lap_index];
                            lap_index += 1;
                            let start = number(lap, 2);
                            let duration = number(lap, 7);
                            let mut candidates = g.laps.iter().filter(|m| {
                                start.is_some()
                                    && duration.is_some()
                                    && number(m, 2) == start
                                    && number(m, 7) == duration
                            });
                            let first = candidates.next();
                            if candidates.next().is_none() {
                                first
                            } else {
                                None
                            }
                        };
                        let same_summary = if session {
                            (b.samples.keys().eq(g.samples.keys())
                                && number(&b.session, 2) == number(&g.session, 2)
                                && number(&b.session, 7) == number(&g.session, 7))
                            .then_some(&g.session)
                        } else {
                            same_lap
                        };
                        if !s.only_bryton {
                            let ids = if session { [16, 17, 64] } else { [15, 16, 63] };
                            for id in ids {
                                let valid = same_summary
                                    .filter(|_| s.only_garmin)
                                    .and_then(|m| number(m, id));
                                if valid.is_some() || native_value(&cells, id, big).is_some() {
                                    set(
                                        &mut cells,
                                        native(id, 2, vec![valid.map_or(255, |v| v as u8)]),
                                    )?;
                                }
                            }
                        }
                        if session {
                            metadata(&mut body, local, cal.dev)?;
                            active[local].clear();
                            cal.write(&mut cells, big)?;
                        } else {
                            for id in [11, 155] {
                                let value = same_lap.and_then(|m| number(m, id));
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
    verify(&header, s, cal)?;
    Ok(header)
}
fn verify(bytes: &[u8], s: &Selection, cal: &Calories) -> Result<()> {
    let mut points = s.points.iter();
    let mut sessions = 0;
    for m in decode(bytes)?.messages {
        if m.num.0 == 20 {
            let p = points
                .next()
                .ok_or_else(|| Error::data("extra output record"))?;
            if number(&m, 253) != Some(p.time) || heart(&m) != p.hr {
                return Err(Error::data("output timestamp or heart rate differs"));
            }
        } else if m.num.0 == 18 {
            sessions += 1;
            for (id, value) in [(11, Some(cal.total)), (196, cal.metabolic)] {
                let dev = m
                    .developer_fields
                    .iter()
                    .find(|f| f.developer_data_index == cal.dev && f.num == id);
                if number(&m, id) != value.map(u32::from)
                    || !dev.is_some_and(
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
