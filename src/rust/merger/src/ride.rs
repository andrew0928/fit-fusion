use crate::{Error, Result};
use fit_inspector::wire::{self, Record};
use rustyfit::{
    Decoder,
    proto::{Message, Value},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub fn number(m: &Message, id: u8) -> Option<u32> {
    match &m.fields.iter().find(|f| f.num == id)?.value {
        Value::Uint8(x) if *x != 255 => Some((*x).into()),
        Value::Uint16(x) if *x != 65535 => Some((*x).into()),
        Value::Uint32(x) if *x != u32::MAX => Some(*x),
        _ => None,
    }
}
pub fn heart(m: &Message) -> Option<u8> {
    number(m, 3)
        .and_then(|v| u8::try_from(v).ok())
        .filter(|&v| v > 0)
}
pub struct Ride {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub hash: String,
    pub device: &'static str,
    pub samples: BTreeMap<u32, Option<u8>>,
    pub boundaries: BTreeSet<u32>,
    pub session: Message,
    pub laps: Vec<Message>,
    pub unknown_fields: BTreeSet<(u16, u8)>,
    pub developer_ids: BTreeSet<u8>,
}
impl Ride {
    pub fn read(path: PathBuf) -> Result<Self> {
        let bytes =
            std::fs::read(&path).map_err(|e| Error::data(format!("{}: {e}", path.display())))?;
        let known: BTreeSet<(u16, u8)> = include_str!("../../../python/fit_fusion/profile.tsv")
            .lines()
            .filter_map(|s| {
                let mut cols = s.split('\t');
                let number = cols.next()?.parse().ok()?;
                let field = cols.nth(1)?.parse().ok()?;
                Some((number, field))
            })
            .collect();
        let mut unknown_fields = BTreeSet::new();
        let mut members = 0;
        let mut developer_ids = BTreeSet::new();
        wire::walk(&bytes, |f| {
            match f.record {
                Record::Header { .. } => members += 1,
                Record::Definition { layout, .. } => {
                    unknown_fields.extend(
                        layout
                            .fields
                            .iter()
                            .filter(|f| {
                                f.developer.is_none() && !known.contains(&(layout.number, f.id))
                            })
                            .map(|f| (layout.number, f.id)),
                    );
                    developer_ids.extend(layout.fields.iter().filter_map(|f| f.developer))
                }
                _ => {}
            }
            Ok(())
        })
        .map_err(|e| Error::data(format!("{}: {e}", path.display())))?;
        if members != 1 {
            return Err(Error::data("expected one FIT activity member"));
        }
        let fit = decode(&bytes)?;
        let mut device = None;
        let mut samples = BTreeMap::new();
        let mut boundaries = BTreeSet::new();
        let mut session = None;
        let mut laps = Vec::new();
        let mut prev = None;
        let mut timer_events = Vec::new();
        for m in fit.messages {
            match m.num.0 {
                0 => {
                    if device.is_some() || number(&m, 0) != Some(4) {
                        return Err(Error::data("expected one activity file_id"));
                    }
                    device = Some(match (number(&m, 1), number(&m, 2)) {
                        (Some(267), Some(2205)) => "bryton",
                        (Some(1), Some(5019)) => "garmin",
                        _ => return Err(Error::data("unrecognized recording device")),
                    });
                }
                18 => {
                    if number(&m, 5) != Some(2) || session.replace(m).is_some() {
                        return Err(Error::data("expected one cycling session"));
                    }
                }
                19 => laps.push(m),
                20 => {
                    let ts = number(&m, 253)
                        .filter(|&n| n >= 0x10000000)
                        .ok_or_else(|| Error::data("record missing absolute UTC timestamp"))?;
                    if prev.is_some_and(|p| p >= ts) {
                        return Err(Error::quality(
                            "ambiguous record order or duplicate timestamp",
                        ));
                    }
                    prev = Some(ts);
                    samples.insert(ts, heart(&m));
                }
                21 if number(&m, 0) == Some(0) => {
                    if let (Some(ts), Some(ty)) = (number(&m, 253), number(&m, 1)) {
                        // A stop includes its final recorded second; a restart begins a new run.
                        if ty == 0 {
                            timer_events.push((ts, true));
                            boundaries.insert(ts);
                        } else if [1, 4, 8, 9].contains(&ty) {
                            timer_events.push((ts, false));
                            boundaries.insert(ts.saturating_add(1));
                        }
                    }
                }
                206 | 207 => {
                    let id = if m.num.0 == 206 { 0 } else { 3 };
                    if let Some(i) = number(&m, id).and_then(|v| u8::try_from(v).ok()) {
                        developer_ids.insert(i);
                    }
                }
                _ => {}
            }
        }
        if samples.len() < 2 {
            return Err(Error::data("need at least two timestamped records"));
        }
        timer_events.sort_by_key(|event| event.0);
        let mut stopped = None;
        for (ts, start) in timer_events
            .into_iter()
            .chain(std::iter::once((u32::MAX, true)))
        {
            if start {
                if let Some(stop) = stopped.take()
                    && stop + 1 < ts
                    && samples.range(stop + 1..ts).next().is_some()
                {
                    return Err(Error::quality(
                        "samples inside a timer pause require review",
                    ));
                }
            } else if stopped.is_none() {
                stopped = Some(ts);
            }
        }
        let hash = crate::hash(&bytes);
        Ok(Self {
            bytes,
            path,
            hash,
            device: device.ok_or_else(|| Error::data("missing file_id"))?,
            samples,
            boundaries,
            session: session.ok_or_else(|| Error::data("missing cycling session"))?,
            laps,
            unknown_fields,
            developer_ids,
        })
    }
    pub fn span(&self) -> (u32, u32) {
        (
            *self.samples.first_key_value().unwrap().0,
            *self.samples.last_key_value().unwrap().0,
        )
    }
}
pub fn decode(bytes: &[u8]) -> Result<rustyfit::proto::FIT> {
    Decoder::builder()
        .expand_components(false)
        .build()
        .decode(bytes)
        .map_err(|e| Error::data(format!("RustyFIT: {e:?}")))?
        .ok_or_else(|| Error::data("empty FIT"))
}
