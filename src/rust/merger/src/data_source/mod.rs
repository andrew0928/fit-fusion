//! Read-only activity sources. Timestamps in public metadata use Unix UTC seconds.
mod directory;
mod garmin;
pub(crate) use garmin::GarminConnect;
mod recording;
use crate::{Error, Result};
pub use directory::FitDirectory;
#[cfg(test)]
pub(crate) use recording::scalar;
pub use recording::{ActivityRecording, NativeSample, Sample, StreamKey, Summary, SummaryKey};
pub(crate) use recording::{decode, number};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    FitDirectory,
    GarminConnect,
    AppleHealthExport,
    Extension(&'static str),
}
impl SourceKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::FitDirectory => "fit_directory",
            Self::GarminConnect => "garmin_connect",
            Self::AppleHealthExport => "apple_health_export",
            Self::Extension(name) => name,
        }
    }
}

pub use fit_inspector::activity::{Device, FitProductName};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RecorderIdentity {
    pub manufacturer: u16,
    pub product: u16,
    pub name: String,
    pub name_source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityMetadata {
    pub source_id: String,
    pub activity_id: String,
    pub device: Device,
    pub product_name: Option<FitProductName>,
    pub sport: u8,
    pub start_utc: i64,
    pub end_utc: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivitySummary {
    pub device: Option<RecorderIdentity>,
    pub source_id: String,
    pub activity_id: String,
    pub start_utc: i64,
    pub end_utc: i64,
}
impl ActivityMetadata {
    pub fn recorder(&self) -> RecorderIdentity {
        let (name, name_source) = self.device.name(self.product_name.as_ref());
        RecorderIdentity {
            manufacturer: self.device.manufacturer,
            product: self.device.product,
            name,
            name_source,
        }
    }
    fn summary(&self) -> ActivitySummary {
        ActivitySummary {
            device: Some(self.recorder()),
            source_id: self.source_id.clone(),
            activity_id: self.activity_id.clone(),
            start_utc: self.start_utc,
            end_utc: self.end_utc,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ActivityQuery {
    All,
    StartsWithin {
        start_utc: i64,
        end_utc: i64,
    },
    Overlapping {
        start_utc: i64,
        end_utc: i64,
        minimum_overlap: f64,
    },
}
impl ActivityQuery {
    pub(crate) fn matches(&self, activity: &ActivityMetadata) -> bool {
        self.matches_interval(activity.start_utc, activity.end_utc)
    }
    pub(crate) fn matches_interval(&self, start: i64, end: i64) -> bool {
        match *self {
            Self::All => true,
            Self::StartsWithin { start_utc, end_utc } => (start_utc..end_utc).contains(&start),
            Self::Overlapping {
                start_utc, end_utc, ..
            } => start_utc < end_utc && start_utc < end && start < end_utc,
        }
    }
}
pub trait DataSource {
    fn id(&self) -> &str;
    fn kind(&self) -> SourceKind;
    fn list_activities(&self, query: &ActivityQuery) -> Result<Vec<ActivitySummary>>;
    fn metadata(&self, activity_id: &str) -> Result<ActivityMetadata>;
    fn open_activity(&self, activity_id: &str) -> Result<ActivityRecording>;
}

// These formats can be declared in a configuration before their readers are installed.
pub(crate) struct DeferredSource {
    pub id: String,
    pub kind: SourceKind,
}
impl DeferredSource {
    fn unavailable(&self) -> Error {
        Error::new(
            "source_unavailable",
            format!(
                "source {}: reader for {} is not implemented",
                self.id,
                self.kind.name()
            ),
            3,
        )
    }
}
impl DataSource for DeferredSource {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> SourceKind {
        self.kind
    }
    fn list_activities(&self, _: &ActivityQuery) -> Result<Vec<ActivitySummary>> {
        Err(self.unavailable())
    }
    fn metadata(&self, _: &str) -> Result<ActivityMetadata> {
        Err(self.unavailable())
    }
    fn open_activity(&self, _: &str) -> Result<ActivityRecording> {
        Err(self.unavailable())
    }
}

#[cfg(test)]
thread_local! {
    pub(crate) static SOURCE_READS:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};
    pub(crate) static PLATFORM_REQUESTS:std::cell::Cell<usize>=const{std::cell::Cell::new(0)};
}
#[cfg(test)]
mod tests;
