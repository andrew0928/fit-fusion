use super::{ActivityMetadata, Device};
use crate::{Error, Result, hash};
use fit_inspector::activity::{self, FIT_EPOCH_UNIX};
use rustyfit::Decoder;
use rustyfit::proto::{FIT, Message, Value};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub(super) struct Inventory {
    pub metadata: Option<ActivityMetadata>,
    pub native: BTreeSet<(u16, u8)>,
    pub developers: BTreeSet<u8>,
}
pub(super) fn inventory(
    bytes: &[u8],
    source: &str,
    path: &Path,
    filter: Option<Device>,
) -> Result<Inventory> {
    let fail = |error| Error::data(format!("{}: {error}", path.display()));
    let mut file = activity::scan(bytes).map_err(fail)?;
    let mut found = Inventory {
        metadata: None,
        native: std::mem::take(&mut file.native),
        developers: std::mem::take(&mut file.developers),
    };
    if !file.is_activity {
        return Ok(found);
    }
    let device = file.device.ok_or_else(|| {
        Error::data(format!(
            "{}: missing activity device identity",
            path.display()
        ))
    })?;
    if filter.is_some_and(|wanted| wanted != device) {
        return Ok(found);
    }
    let Some((start, elapsed)) = file.cycling_session().map_err(fail)? else {
        return Ok(found);
    };
    let start_utc = FIT_EPOCH_UNIX + i64::from(start);
    found.metadata = Some(ActivityMetadata {
        source_id: source.into(),
        activity_id: hash(bytes),
        device,
        product_name: file.product_name,
        sport: 2,
        start_utc,
        end_utc: start_utc + i64::from(elapsed.div_ceil(1000)),
    });
    Ok(found)
}

pub struct ActivityRecording {
    pub(crate) path: PathBuf,
    pub(crate) bytes: Vec<u8>,
    pub(crate) metadata: ActivityMetadata,
    pub(crate) fit: FIT,
    pub(crate) native: BTreeSet<(u16, u8)>,
    pub(crate) developers: BTreeSet<u8>,
}

pub struct NativeSample {
    pub timestamp_utc: i64,
    pub raw: Option<serde_json::Value>,
}

pub struct Sample {
    pub timestamp_utc: i64,
    /// Encoded integer in the FIT field's native scale and units.
    pub raw: Option<i64>,
}

/// Common record field numbers for the integer convenience API. Use read_native_stream for arbitrary FIT fields.
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum StreamKey {
    Latitude = 0,
    Longitude = 1,
    Altitude = 2,
    HeartRate = 3,
    Cadence = 4,
    Distance = 5,
    Speed = 6,
    Power = 7,
    Temperature = 13,
    EnhancedSpeed = 73,
    EnhancedAltitude = 78,
}

#[derive(Clone, Copy)]
pub enum SummaryKey {
    SessionTotalCalories,
    SessionMetabolicCalories,
    LapTotalCalories,
}

pub struct Summary {
    pub start_utc: Option<i64>,
    pub end_utc: Option<i64>,
    pub raw: Option<i64>,
}

pub(crate) fn scalar(message: &Message, field: u8) -> Option<i64> {
    match &message.fields.iter().find(|f| f.num == field)?.value {
        Value::Uint8(x) if *x != u8::MAX => Some((*x).into()),
        Value::Uint16(x) if *x != u16::MAX => Some((*x).into()),
        Value::Uint32(x) if *x != u32::MAX => Some((*x).into()),
        Value::Int8(x) if *x != i8::MAX => Some((*x).into()),
        Value::Int16(x) if *x != i16::MAX => Some((*x).into()),
        Value::Int32(x) if *x != i32::MAX => Some((*x).into()),
        _ => None,
    }
}
impl ActivityRecording {
    pub(super) fn read(path: &Path, source: &str) -> Result<Self> {
        #[cfg(test)]
        super::SOURCE_READS.with(|n| n.set(n.get() + 1));
        let bytes = fs::read(path)
            .map_err(|e| Error::new("source_io", format!("{}: {e}", path.display()), 3))?;
        Self::from_fit_bytes(source, path, bytes)
    }
    /// Decode once and retain the exact input for provenance and independent readers.
    pub fn from_fit_bytes(source: &str, path: &Path, bytes: Vec<u8>) -> Result<Self> {
        let scan = inventory(&bytes, source, path, None)?;
        let metadata = scan.metadata.ok_or_else(|| {
            Error::data(format!("{}: expected a cycling activity", path.display()))
        })?;
        let fit = decode(&bytes)?;
        if fit
            .messages
            .iter()
            .filter(|m| m.num.0 == 20)
            .any(|m| number(m, 253).is_none_or(|t| t < 0x10000000))
        {
            return Err(Error::data(format!(
                "{}: record missing absolute UTC timestamp",
                path.display()
            )));
        }
        Ok(Self {
            path: path.into(),
            bytes,
            metadata,
            fit,
            native: scan.native,
            developers: scan.developers,
        })
    }
    pub fn metadata(&self) -> &ActivityMetadata {
        &self.metadata
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn read_stream(&self, field: StreamKey) -> Option<Vec<Sample>> {
        let native_field = field as u8;
        self.native.contains(&(20, native_field)).then(|| {
            self.fit
                .messages
                .iter()
                .filter(|m| m.num.0 == 20)
                .filter_map(|m| {
                    Some(Sample {
                        timestamp_utc: FIT_EPOCH_UNIX + i64::from(number(m, 253)?),
                        raw: scalar(m, native_field),
                    })
                })
                .collect()
        })
    }
    /// A lossless wire-based field read decoded into scalar, array or string values.
    /// The outer None means the file has no such native record field.
    pub fn read_native_stream(&self, field: u8) -> Result<Option<Vec<NativeSample>>> {
        if !self.native.contains(&(20, field)) {
            return Ok(None);
        }
        let times = self
            .fit
            .messages
            .iter()
            .filter(|m| m.num.0 == 20)
            .filter_map(|m| number(m, 253))
            .collect::<Vec<_>>();
        let records =
            crate::native::read_records(&self.bytes, times.iter(), &BTreeSet::from([field]))?;
        Ok(Some(
            records
                .into_iter()
                .map(|(time, values)| NativeSample {
                    timestamp_utc: FIT_EPOCH_UNIX + i64::from(time),
                    raw: values.get(&field).map(|v| v.decoded()),
                })
                .collect(),
        ))
    }
    pub fn read_summary(&self, key: SummaryKey) -> Option<Vec<Summary>> {
        let (message, field) = match key {
            SummaryKey::SessionTotalCalories => (18, 11),
            SummaryKey::SessionMetabolicCalories => (18, 196),
            SummaryKey::LapTotalCalories => (19, 11),
        };
        self.native.contains(&(message, field)).then(|| {
            self.fit
                .messages
                .iter()
                .filter(|m| m.num.0 == message)
                .map(|m| {
                    let start_utc = number(m, 2).map(|t| FIT_EPOCH_UNIX + i64::from(t));
                    let end_utc = start_utc
                        .zip(number(m, 7))
                        .map(|(start, ms)| start + i64::from(ms.div_ceil(1000)));
                    Summary {
                        start_utc,
                        end_utc,
                        raw: scalar(m, field),
                    }
                })
                .collect()
        })
    }
}

pub(crate) fn number(m: &Message, id: u8) -> Option<u32> {
    u32::try_from(scalar(m, id)?).ok()
}
#[cfg(test)]
thread_local! {pub(super) static DECODE_COUNT:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};}
pub(crate) fn decode(bytes: &[u8]) -> Result<rustyfit::proto::FIT> {
    #[cfg(test)]
    DECODE_COUNT.with(|c| c.set(c.get() + 1));
    Decoder::builder()
        .expand_components(false)
        .build()
        .decode(bytes)
        .map_err(|e| Error::data(format!("RustyFIT: {e:?}")))?
        .ok_or_else(|| Error::data("empty FIT"))
}
