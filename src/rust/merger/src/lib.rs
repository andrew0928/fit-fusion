pub mod config;
pub mod data_source;
mod output;
mod policy;
mod quality;
mod ride;
use data_source::{ActivityRecording, FitDirectory};
use policy::MatchedRide;
use quality::Selection;
use ride::Ride;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
const VERSION: &str = "offline-dual-fit-v1";
#[derive(Debug)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
    pub details: Value,
    pub exit: u8,
}
impl Error {
    fn new(code: &'static str, message: impl Into<String>, exit: u8) -> Self {
        Self {
            code,
            message: message.into(),
            details: Value::Null,
            exit,
        }
    }
    pub fn data(s: impl Into<String>) -> Self {
        Self::new("invalid_data", s, 3)
    }
    pub fn quality(s: impl Into<String>) -> Self {
        Self::new("quality_review", s, 3)
    }
    pub fn usage(s: impl Into<String>) -> Self {
        Self::new("usage", s, 2)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("io_error", e.to_string(), 3)
    }
}
pub type Result<T> = std::result::Result<T, Error>;
fn hash(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}
fn provenance(s: &Selection) -> Vec<Value> {
    let mut ranges = vec![];
    let mut i = 0;
    while i < s.points.len() {
        let p = &s.points[i];
        let mut j = i + 1;
        while j < s.points.len()
            && s.points[j].source == p.source
            && s.points[j].time == s.points[j - 1].time + 1
        {
            j += 1;
        }
        ranges.push(json!({"start_fit_seconds":p.time,"end_fit_seconds":s.points[j-1].time,"source":p.source,"field":"record.heart_rate"}));
        i = j;
    }
    ranges
}
fn write_result(out: &Path, bytes: &[u8], report: &Value) -> Result<bool> {
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let contents = serde_json::to_vec_pretty(report).map_err(std::io::Error::other)?;
    let reuse = || -> Result<bool> {
        if fs::read(out.join("merged.fit")).ok().as_deref() == Some(bytes)
            && fs::read(out.join("merge-report.json")).ok().as_deref() == Some(contents.as_slice())
        {
            Ok(true)
        } else {
            Err(Error::new(
                "output_conflict",
                "destination exists with other contents; choose a new directory",
                3,
            ))
        }
    };
    if out.try_exists()? {
        return reuse();
    }
    let tmp = tempfile::tempdir_in(parent)?;
    for (name, data) in [("merged.fit", bytes), ("merge-report.json", &contents)] {
        let mut f = fs::File::create(tmp.path().join(name))?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    match renameat_with(CWD, tmp.path(), CWD, out, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(false),
        Err(rustix::io::Errno::EXIST) => reuse(),
        Err(e) => Err(std::io::Error::from(e).into()),
    }
}
pub fn merge_files(inputs: [PathBuf; 2], out: PathBuf) -> Result<Value> {
    let [first, second] = inputs;
    merge_recordings(
        FitDirectory::open_file(&first)?,
        FitDirectory::open_file(&second)?,
        &out,
    )
}

/// Merge two already-open recordings with the production dual-file defaults.
/// A directory caller can pass DataSource::open_activity results without opening paths again.
pub fn merge_recordings(
    first: ActivityRecording,
    second: ActivityRecording,
    out: &Path,
) -> Result<Value> {
    let settings = config::Settings::embedded()?;
    let pair = MatchedRide::new(first, second)?;
    let selected = settings.merge_policy().select(&pair)?;
    let (b, g) = (&pair.primary, &pair.secondary);
    let (s, c) = (&selected.heart, &selected.calories);
    let bytes = output::build(b, g, s, c)?;
    let source = |r: &Ride| {
        let mut info = json!({"path":r.path,"sha256":r.hash,"record_interval_fit_seconds":r.span(),"unknown_native_message_field_pairs":r.unknown_fields});
        if !r.source_id.is_empty() {
            info["source_id"] = json!(r.source_id);
        }
        info
    };
    let report = json!({"schema_version":1,"rules_version":VERSION,"inputs":{"bryton":source(b),"garmin":source(g)},
        "selection":settings.selection(),"overlap":{"record_ratios":selected.record_overlap,"session_ratios":selected.session_overlap},
        "heart_rate":s.report,"heart_rate_sources":provenance(s),"primary_timeline":"bryton",
        "calories":{"source":"garmin","bryton_original":crate::data_source::number(&b.session,11),"total_calories":c.total,"metabolic_calories":c.metabolic,
        "active_calories":c.metabolic.and_then(|v|c.total.checked_sub(v)),"developer_index":c.dev,"application_id":hex::encode(output::APP_ID)},
        "output_sha256":hash(&bytes)});
    let reused = write_result(out, &bytes, &report)?;
    Ok(
        json!({"schema_version":1,"status":"exported","code":"ok","reused":reused,"artifacts":{"fit":out.join("merged.fit"),
        "report":out.join("merge-report.json")},"next_action":null}),
    )
}
