use crate::data_source::number;
use crate::data_source::{ActivityRecording, RecorderIdentity};
use crate::{Error, Result};
use rustyfit::proto::Message;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

pub fn heart(m: &Message) -> Option<u8> {
    number(m, 3)
        .and_then(|v| u8::try_from(v).ok())
        .filter(|&v| v > 0)
}
pub struct Ride {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub hash: String,
    pub recorder: RecorderIdentity,
    pub session_utc: (i64, i64),
    pub source_id: String,
    pub records: BTreeMap<u32, Message>,
    pub samples: BTreeMap<u32, Option<u8>>,
    pub boundaries: BTreeSet<u32>,
    pub session: Message,
    pub laps: Vec<Message>,
    pub unknown_fields: BTreeSet<(u16, u8)>,
    pub developer_ids: BTreeSet<u8>,
}
impl Ride {
    pub fn from_recording(recording: ActivityRecording) -> Result<Self> {
        let ActivityRecording {
            path,
            bytes,
            metadata,
            fit,
            native,
            developers: mut developer_ids,
        } = recording;
        let known: BTreeSet<(u16, u8)> = include_str!("../../../python/fit_fusion/profile.tsv")
            .lines()
            .filter_map(|s| {
                let mut cols = s.split('\t');
                let number = cols.next()?.parse().ok()?;
                let field = cols.nth(1)?.parse().ok()?;
                Some((number, field))
            })
            .collect();
        let unknown_fields = native.difference(&known).copied().collect();
        let mut records = BTreeMap::new();
        let mut samples = BTreeMap::new();
        let mut boundaries = BTreeSet::new();
        let mut session = None;
        let mut laps = Vec::new();
        let mut prev = None;
        let mut timer_events = Vec::new();
        for m in fit.messages {
            match m.num.0 {
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
                    records.insert(ts, m);
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
            recorder: metadata.recorder(),
            session_utc: (metadata.start_utc, metadata.end_utc),
            source_id: metadata.source_id,
            records,
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
