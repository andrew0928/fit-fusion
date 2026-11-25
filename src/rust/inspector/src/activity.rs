//! FIT session and recording-device metadata shared by directory readers.
use crate::wire::{self, Record};
use std::{collections::BTreeSet, io};

pub const FIT_EPOCH_UNIX: i64 = 631065600;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Device {
    pub manufacturer: u16,
    pub product: u16,
}
impl Device {
    pub const RIDER_460: Self = Self {
        manufacturer: 267,
        product: 2205,
    };
    pub const CIRQA: Self = Self {
        manufacturer: 1,
        product: 5019,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitProductName {
    FileId(String),
    Creator(String),
}

impl Device {
    pub fn name(self, product_name: Option<&FitProductName>) -> (String, &'static str) {
        let Device {
            manufacturer,
            product,
        } = self;
        match product_name {
            Some(FitProductName::FileId(s)) => (s.clone(), "file_id.product_name"),
            Some(FitProductName::Creator(s)) => (s.clone(), "device_info.product_name"),
            None => match self {
                Device {
                    manufacturer: 267,
                    product: 2004,
                } => ("Bryton Rider 15 Neo".into(), "model_lookup"),
                Device::RIDER_460 => ("Bryton Rider 460".into(), "model_lookup"),
                Device::CIRQA => ("Garmin CIRQA".into(), "model_lookup"),
                _ => (format!("FIT {manufacturer}/{product}"), "fit_ids"),
            },
        }
    }
}

#[derive(Default)]
pub struct ActivityFile {
    pub is_activity: bool,
    pub device: Option<Device>,
    pub product_name: Option<FitProductName>,
    sessions: Vec<(Option<u32>, Option<u32>, Option<u32>)>,
    pub native: BTreeSet<(u16, u8)>,
    pub developers: BTreeSet<u8>,
}

impl ActivityFile {
    /// Start in FIT epoch seconds, elapsed time in milliseconds.
    pub fn cycling_session(&self) -> io::Result<Option<(u32, u32)>> {
        if self.sessions.is_empty() {
            return Err(wire::invalid("missing activity session"));
        }
        if !self.sessions.iter().any(|s| s.0 == Some(2)) {
            return Ok(None);
        }
        let [(Some(2), Some(start), Some(elapsed))] = self.sessions.as_slice() else {
            return Err(wire::invalid(
                "expected one cycling session with start and elapsed time",
            ));
        };
        if *elapsed == 0 {
            return Err(wire::invalid(
                "expected one activity with a positive duration",
            ));
        }
        Ok(Some((*start, *elapsed)))
    }
}

pub fn scan(bytes: &[u8]) -> io::Result<ActivityFile> {
    let mut found = ActivityFile::default();
    let mut members = 0;
    let mut file_name = None;
    let mut creator_names = Vec::new();
    let mut file_ids = 0;
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
            Record::Data { layout, values, .. } if matches!(layout.number, 0 | 18 | 23) => {
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
                let text = |id| {
                    let (_, raw) =
                        layout.fields.iter().zip(&values).find(|(f, _)| {
                            f.developer.is_none() && f.id == id && f.base == Some(7)
                        })?;
                    let value = std::str::from_utf8(raw.split(|b| *b == 0).next()?)
                        .ok()?
                        .trim();
                    (!value.is_empty()).then(|| value.to_owned())
                };
                if layout.number == 23
                    && uint(0) == Some(0)
                    && let Some(name) = text(27)
                {
                    creator_names.push((uint(2), uint(4), name));
                }
                if layout.number == 0 {
                    file_ids += 1;
                }
                if layout.number == 0 && uint(0) == Some(4) {
                    found.is_activity = true;
                    file_name = text(8);
                    found.device = uint(1).zip(uint(2)).and_then(|(maker, product)| {
                        Some(Device {
                            manufacturer: u16::try_from(maker).ok()?,
                            product: u16::try_from(product).ok()?,
                        })
                    });
                } else if layout.number == 18 {
                    found.sessions.push((uint(5), uint(2), uint(7)));
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    if members != 1 || file_ids != 1 {
        return Err(wire::invalid("expected one FIT member and file_id"));
    }

    found.product_name = file_name.map(FitProductName::FileId).or_else(|| {
        let device = found.device?;
        creator_names
            .into_iter()
            .find(|(maker, product, _)| {
                maker.is_none_or(|n| n == u32::from(device.manufacturer))
                    && product.is_none_or(|n| n == u32::from(device.product))
            })
            .map(|(_, _, s)| FitProductName::Creator(s))
    });
    Ok(found)
}
