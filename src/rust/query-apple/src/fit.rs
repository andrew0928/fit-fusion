//! Encode a cycling activity from observed samples and explicitly mark its origin.
use crate::{
    Result, bad,
    health::{Metric, Observations, Workout},
};
use fit_inspector::{
    activity::FIT_EPOCH_UNIX,
    wire::{self, Field},
    write::{self, Cell, native},
};
use jiff::Timestamp;

fn u8f(id: u8, base: u8, v: u8) -> Cell {
    native(id, base, vec![v])
}
fn u16f(id: u8, v: u16) -> Cell {
    native(id, 0x84, v.to_le_bytes().to_vec())
}
fn u32f(id: u8, v: u32) -> Cell {
    native(id, 0x86, v.to_le_bytes().to_vec())
}
fn string(id: u8, s: &str) -> Result<Cell> {
    if s.len() > 254 || s.contains('\0') {
        return Err(bad("FIT string exceeds field size or contains NUL"));
    }
    let mut bytes = s.as_bytes().to_vec();
    bytes.push(0);
    Ok(native(id, 7, bytes))
}
fn seconds(t: Timestamp) -> Result<u32> {
    let n = u32::try_from(t.as_second() - FIT_EPOCH_UNIX)?;
    if n == u32::MAX {
        return Err(bad("invalid FIT date"));
    }
    Ok(n)
}
fn scaled(v: f64, scale: f64, offset: f64, invalid: u32) -> Result<u32> {
    let x = ((v + offset) * scale).round();
    if !x.is_finite() || x < 0. || x >= f64::from(invalid) {
        return Err(bad("measurement outside FIT range"));
    }
    Ok(x as u32)
}
fn dev(id: u8, bytes: Vec<u8>) -> Cell {
    Cell {
        def: Field {
            id,
            base: None,
            size: bytes.len() as u8,
            developer: Some(0),
        },
        data: bytes,
    }
}
struct Encoder {
    body: Vec<u8>,
    definition: Vec<u8>,
}
impl Encoder {
    fn msg(&mut self, num: u16, mut fields: Vec<Cell>) -> Result<()> {
        fields.sort_by_key(|f| (f.def.developer, f.def.id));
        let def = write::definition(0, num, false, &fields)?;
        if def != self.definition {
            self.body.extend(&def);
            self.definition = def;
        }
        self.body.push(0);
        for field in fields {
            self.body.extend(field.data);
        }
        Ok(())
    }
    fn finish(self) -> Result<Vec<u8>> {
        let len = u32::try_from(self.body.len())?;
        let mut file = vec![14, 0x20];
        file.extend(2100u16.to_le_bytes());
        file.extend(len.to_le_bytes());
        file.extend(b".FIT");
        file.extend(wire::crc(&file).to_le_bytes());
        file.extend(self.body);
        file.extend(wire::crc(&file).to_le_bytes());
        Ok(file)
    }
}
pub fn encode(w: &Workout, obs: &Observations) -> Result<Vec<u8>> {
    let mut e = Encoder {
        body: Vec::new(),
        definition: Vec::new(),
    };
    let start = seconds(w.start_utc)?;
    let end = seconds(w.end_utc)?;
    let provenance = format!(
        "Apple Health conversion / {}{}",
        w.source_name,
        w.device_name
            .as_ref()
            .map(|d| format!(" / {d}"))
            .unwrap_or_default()
    );
    e.msg(
        0,
        vec![
            u8f(0, 0, 4),
            u16f(1, 255),
            u16f(2, 0),
            u32f(4, start),
            string(8, &provenance)?,
        ],
    )?;
    // Fixed application identity names our conversion fields across activities and revisions.
    e.msg(
        207,
        vec![native(1, 13, b"FitFusionApple01".to_vec()), u8f(3, 2, 0)],
    )?;
    for (id, base, name, units) in [
        (0, 0x86, "subsecond_ns", "ns"),
        (1, 0x89, "distance_delta", "m"),
        (2, 0x86, "sample_interval_ms", "ms"),
    ] {
        e.msg(
            206,
            vec![
                u8f(0, 2, 0),
                u8f(1, 2, id),
                u8f(2, 2, base),
                string(3, name)?,
                string(8, units)?,
            ],
        )?;
    }
    for (&t, slots) in &obs.rows {
        for slot in slots {
            let mut cells = vec![u32f(253, seconds(t)?)];
            if t.subsec_nanosecond() != 0 {
                cells.push(dev(
                    0,
                    (t.subsec_nanosecond() as u32).to_le_bytes().to_vec(),
                ));
            }
            for (&m, &v) in slot {
                let f = match m {
                    Metric::Lat | Metric::Lon => {
                        let semicircles = if v == 180. {
                            i32::MIN
                        } else {
                            (v * (2147483648. / 180.)).round() as i32
                        };
                        if semicircles == i32::MAX {
                            return Err(bad("coordinate rounds to FIT invalid value"));
                        }
                        native(
                            if m == Metric::Lat { 0 } else { 1 },
                            0x85,
                            semicircles.to_le_bytes().to_vec(),
                        )
                    }
                    Metric::Heart | Metric::Cadence => u8f(
                        if m == Metric::Heart { 3 } else { 4 },
                        2,
                        scaled(v, 1., 0., 255)? as u8,
                    ),
                    Metric::Power => u16f(7, scaled(v, 1., 0., 65535)? as u16),
                    Metric::Speed => u32f(73, scaled(v, 1000., 0., u32::MAX)?),
                    Metric::Altitude => u32f(78, scaled(v, 5., 500., u32::MAX)?),
                    Metric::DistanceDelta => dev(1, v.to_le_bytes().to_vec()),
                    Metric::IntervalMs => {
                        dev(2, scaled(v, 1., 0., u32::MAX)?.to_le_bytes().to_vec())
                    }
                };
                cells.push(f);
            }
            e.msg(20, cells)?;
        }
    }
    let elapsed = u32::try_from(w.end_utc.as_millisecond() - w.start_utc.as_millisecond())?;
    let timer = w
        .timer_s
        .map(|s| scaled(s, 1000., 0., u32::MAX))
        .transpose()?;
    for n in [19, 18] {
        let mut fields = vec![
            u32f(253, end),
            u16f(254, 0),
            u32f(2, start),
            u32f(7, elapsed),
            u8f(if n == 18 { 5 } else { 25 }, 0, 2),
        ];
        if let Some(v) = timer {
            fields.push(u32f(8, v));
        }
        if let Some(v) = w.distance_m {
            fields.push(u32f(9, scaled(v, 100., 0., u32::MAX)?));
        }
        if let Some(v) = w.kcal {
            fields.push(u16f(11, scaled(v, 1., 0., 65535)? as u16));
        }
        if n == 18 {
            fields.extend([u16f(25, 0), u16f(26, 1)]);
        }
        e.msg(n, fields)?;
    }
    let mut activity = vec![
        u32f(253, end),
        u16f(1, 1),
        u8f(2, 0, 0),
        u8f(3, 0, 26),
        u8f(4, 0, 1),
    ];
    if let Some(v) = timer {
        activity.push(u32f(0, v));
    }
    e.msg(34, activity)?;
    e.finish()
}
