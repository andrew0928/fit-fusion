use super::{ActivityMetadata, Device};
use crate::{Error, Result, hash};
use fit_inspector::wire::{self, Record};
use rustyfit::Decoder;
use rustyfit::proto::{FIT, Message, Value};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

const FIT_EPOCH_UNIX: i64 = 631065600;

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
    let mut found = Inventory::default();
    let mut members = 0;
    let mut device = None;
    let mut activity_file = false;
    let mut file_ids = 0;
    let mut sessions = Vec::new();
    wire::walk(bytes, |frame| {
        match frame.record {
            Record::Header { .. } => members += 1,
            Record::Definition { layout, .. } => {
                for f in &layout.fields {
                    if let Some(id) = f.developer {
                        found.developers.insert(id);
                    } else {
                        found.native.insert((layout.number, f.id));
                    }
                }
            }
            Record::Data { layout, values, .. } if matches!(layout.number, 0 | 18) => {
                let uint = |id| {
                    let (field, bytes) = layout
                        .fields
                        .iter()
                        .zip(values.iter())
                        .find(|(f, _)| f.developer.is_none() && f.id == id)?;
                    let width = wire::width(field.base?)?;
                    if width != bytes.len() || width > 4 || bytes.iter().all(|&b| b == 255) {
                        return None;
                    }
                    Some(wire::unsigned(bytes, layout.big) as u32)
                };
                if layout.number == 0 {
                    file_ids += 1;
                }
                if layout.number == 0 && uint(0) == Some(4) {
                    activity_file = true;
                    device = uint(1).zip(uint(2)).and_then(|(maker, product)| {
                        Some(Device {
                            manufacturer: u16::try_from(maker).ok()?,
                            product: u16::try_from(product).ok()?,
                        })
                    });
                } else if layout.number == 18 {
                    sessions.push((uint(5), uint(2), uint(7)));
                }
            }
            _ => {}
        }
        Ok(())
    })
    .map_err(|e| Error::data(format!("{}: {e}", path.display())))?;
    if members != 1 || file_ids != 1 {
        return Err(Error::data(format!(
            "{}: expected one FIT member and file_id",
            path.display()
        )));
    }
    if !activity_file {
        return Ok(found);
    }
    let device = device.ok_or_else(|| {
        Error::data(format!(
            "{}: missing activity device identity",
            path.display()
        ))
    })?;
    if filter.is_some_and(|wanted| device != wanted) {
        return Ok(found);
    }
    if sessions.is_empty() {
        return Err(Error::data(format!(
            "{}: missing activity session",
            path.display()
        )));
    }
    if !sessions.iter().any(|s| s.0 == Some(2)) {
        return Ok(found);
    }
    let [(Some(2), Some(start), Some(elapsed))] = sessions.as_slice() else {
        return Err(Error::data(format!(
            "{}: expected one cycling session with start and elapsed time",
            path.display()
        )));
    };
    if *elapsed == 0 {
        return Err(Error::data(format!(
            "{}: expected one activity with a positive duration",
            path.display()
        )));
    }
    let start_utc = FIT_EPOCH_UNIX + i64::from(*start);
    found.metadata = Some(ActivityMetadata {
        source_id: source.into(),
        activity_id: hash(bytes),
        device,
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
