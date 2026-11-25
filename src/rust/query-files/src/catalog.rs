//! Read-only discovery of local cycling activities. IDs identify FIT contents.
use fit_inspector::{
    activity::{self, Device, FIT_EPOCH_UNIX},
    inventory,
};
use jiff::{Timestamp, civil::Date, tz::TimeZone};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs, io,
    ops::Range,
    path::{Path, PathBuf},
};

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub struct Query {
    pub days: Option<Range<Date>>,
    pub timezone: TimeZone,
    pub device: Option<Device>,
    pub id: Option<String>,
}

#[derive(Serialize)]
pub struct Activity {
    pub source_id: PathBuf,
    pub id: String,
    pub path: PathBuf,
    pub device: Option<Recorder>,
    pub activity_name: Option<String>,
    pub device_name: Option<String>,
    pub sport: &'static str,
    pub start_utc: Timestamp,
    pub end_utc: Timestamp,
}
#[derive(Serialize)]
pub struct Recorder {
    manufacturer: u16,
    product: u16,
    pub name: String,
    name_source: &'static str,
}

#[derive(Serialize)]
pub struct FileError {
    #[serde(serialize_with = "diagnostic_path")]
    pub path: PathBuf,
    pub message: String,
}

fn diagnostic_path<S: serde::Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&path.to_string_lossy())
}

pub struct Listing {
    pub activities: Vec<Activity>,
    pub errors: Vec<FileError>,
}

impl Activity {
    fn inspect(path: &Path, root: &Path, query: &Query) -> io::Result<Option<Self>> {
        if path.to_str().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "FIT path cannot be represented in JSON as UTF-8",
            ));
        }
        let bytes = fs::read(path)?;
        let id = digest(&bytes);
        if query.id.as_ref().is_some_and(|want| *want != id) {
            return Ok(None);
        }
        let file = activity::scan(&bytes)?;
        if !file.is_activity || query.device.is_some_and(|d| Some(d) != file.device) {
            return Ok(None);
        }
        let Some((start, elapsed_ms)) = file.cycling_session()? else {
            return Ok(None);
        };
        let start_utc = Timestamp::from_second(FIT_EPOCH_UNIX + i64::from(start))
            .expect("FIT timestamp fits jiff");
        if let Some(days) = &query.days
            && !days.contains(&start_utc.to_zoned_datetime(query.timezone.clone()).date())
        {
            return Ok(None);
        }
        let end_utc =
            Timestamp::from_millisecond(start_utc.as_millisecond() + i64::from(elapsed_ms))
                .expect("FIT timestamp fits jiff");
        let device = file.device.map(|device| {
            let (name, name_source) = device.name(file.product_name.as_ref());
            Recorder {
                manufacturer: device.manufacturer,
                product: device.product,
                name,
                name_source,
            }
        });
        Ok(Some(Self {
            source_id: root.into(),
            id,
            path: path.into(),
            device,
            activity_name: None, // Plain activity FIT has no standard activity title.
            device_name: file.device_name(),
            sport: "cycling",
            start_utc,
            end_utc,
        }))
    }

    pub fn original(&self) -> io::Result<Vec<u8>> {
        let bytes = fs::read(&self.path)?;
        if digest(&bytes) != self.id {
            return Err(io::Error::other(
                "source changed since discovery; run list again",
            ));
        }
        Ok(bytes)
    }

    /// Use the same validity, developer definitions and compressed clock as fit-inspector.
    pub fn streams(&self) -> io::Result<Vec<Value>> {
        inventory::record_streams(&self.original()?, &self.path.display().to_string())
    }
}

pub fn list(directory: &Path, query: &Query, output: Option<&Path>) -> io::Result<Listing> {
    let root = directory.canonicalize()?;
    let excluded = output
        .and_then(|p| p.canonicalize().ok())
        .filter(|p| p != &root);
    let mut directories = vec![root.clone()];
    let mut candidates = Vec::new();
    let mut errors = Vec::new();
    while let Some(dir) = directories.pop() {
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(e) if dir == root => return Err(e),
            Err(e) => {
                errors.push(FileError {
                    path: dir,
                    message: e.to_string(),
                });
                continue;
            }
        };
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    errors.push(FileError {
                        path: dir.clone(),
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() && Some(&path) != excluded.as_ref() => {
                    directories.push(path)
                }
                Ok(kind)
                    if kind.is_file()
                        && path
                            .extension()
                            .is_some_and(|ext| ext.eq_ignore_ascii_case("fit")) =>
                {
                    candidates.push(path)
                }
                Ok(_) => {}
                Err(e) => errors.push(FileError {
                    path,
                    message: e.to_string(),
                }),
            }
        }
    }
    // Keep the lexicographically first original when multiple paths have the same content.
    candidates.sort_unstable();
    let mut seen = HashSet::new();
    let mut activities = Vec::new();
    for path in candidates {
        match Activity::inspect(&path, &root, query) {
            Ok(Some(activity)) if seen.insert(activity.id.clone()) => activities.push(activity),
            Ok(_) => {}
            Err(e) => errors.push(FileError {
                path,
                message: e.to_string(),
            }),
        }
    }
    activities.sort_by(|a, b| (a.start_utc, &a.id).cmp(&(b.start_utc, &b.id)));
    errors.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(Listing { activities, errors })
}
