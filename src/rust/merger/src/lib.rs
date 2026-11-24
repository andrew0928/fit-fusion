pub mod config;
pub mod data_source;
mod output;
mod policy;
mod quality;
mod ride;
pub mod workflow;
use data_source::{ActivityRecording, FitDirectory};
use policy::MatchedRide;
use quality::Selection;
use ride::Ride;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
const VERSION: &str = "configured-fit-v2";
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
    manual_pair(&config::Settings::embedded()?, first, second, out)
}
fn manual_pair(
    settings: &config::Settings,
    first: ActivityRecording,
    second: ActivityRecording,
    out: &Path,
) -> Result<Value> {
    use data_source::Device;
    let (b, g) = match (first.metadata.device, second.metadata.device) {
        (Device::RIDER_460, Device::CIRQA) => (first, second),
        (Device::CIRQA, Device::RIDER_460) => (second, first),
        _ => return Err(Error::data("expected one Rider 460 and one CIRQA")),
    };
    let primary_id = settings.primary_source().id().to_owned();
    let mut rides = BTreeMap::from([(primary_id.clone(), Ride::from_recording(b)?)]);
    let ids = workflow::required(settings);
    if ids.len() > 1 {
        return Err(Error::usage(
            "use date mode to identify multiple configured secondary sources",
        ));
    }
    if let Some(id) = ids.first() {
        if settings.device(id) != Some(Device::CIRQA) {
            return Err(Error::usage(
                "second input must match the configured CIRQA source",
            ));
        }
        rides.insert(id.clone(), Ride::from_recording(g)?);
    }
    export(settings, &MatchedRide { primary_id, rides }, out, false)
}
fn export(
    settings: &config::Settings,
    pair: &MatchedRide,
    out: &Path,
    revision_path: bool,
) -> Result<Value> {
    let selected = settings.merge_policy().select(pair)?;
    let b = pair.primary();
    let bytes = output::build(b, &selected)?;
    let s = &selected.heart;
    let mut inputs = BTreeMap::new();
    for (id, r) in &pair.rides {
        let mut info = json!({"sha256":r.hash,"device":{"manufacturer":r.device.manufacturer,"product":r.device.product},"record_interval_fit_seconds":r.span(),"unknown_native_message_field_pairs":r.unknown_fields});
        if !revision_path {
            info["path"] = json!(r.path);
        }
        info["source_id"] = json!(if r.source_id.is_empty() {
            id
        } else {
            &r.source_id
        });
        inputs.insert(id, info);
    }
    let calories=selected.calories.as_ref().map(|c|json!({
        "source":selected.calorie_source.and_then(|r|pair.rides.iter().find(|(_,v)|std::ptr::eq(*v,r)).map(|(id,_)|id)),
        "bryton_original":data_source::number(&b.session,11),"total_calories":c.total,"metabolic_calories":c.metabolic,
        "active_calories":c.active(),"developer_index":selected.write_total.then_some(c.dev),
        "application_id":hex::encode(output::APP_ID)
    }));
    let hr_ids = settings
        .sources_for("record.heart_rate")
        .map(|s| s.id())
        .collect::<Vec<_>>();
    let mut hr_ranges = provenance(s);
    for range in &mut hr_ranges {
        let id = match range["source"].as_str() {
            Some("bryton") => hr_ids.first(),
            Some("garmin") => hr_ids.get(1),
            _ => None,
        };
        if let Some(id) = id {
            range["source"] = json!(id);
        }
    }
    let field_sources: BTreeMap<_, _> = selected
        .fields
        .iter()
        .map(|f| (f.name, &f.ranges))
        .collect();
    let first_overlap = selected.overlaps.values().next();
    let report = json!({"schema_version":1,"rules_version":VERSION,"inputs":inputs,"primary_timeline":pair.primary_id,
        "selection":settings.selection(),"overlap":{"record_ratios":first_overlap.map(|r|r[0]),"session_ratios":first_overlap.map(|r|r[1]),"by_source":selected.overlaps},
        "heart_rate":s.report,"heart_rate_sources":hr_ranges,"stream_sources":field_sources,"calories":calories,"output_sha256":hash(&bytes)});
    let hashes: BTreeMap<_, _> = pair.rides.iter().map(|(id, r)| (id, &r.hash)).collect();
    let revision = hash(
        &serde_json::to_vec(&(VERSION, hashes, settings.selection()))
            .map_err(std::io::Error::other)?,
    );
    let out = if revision_path {
        out.join(&b.hash).join(&revision)
    } else {
        out.to_owned()
    };
    let reused = write_result(&out, &bytes, &report)?;
    Ok(
        json!({"schema_version":1,"status":"exported","code":"ok","revision_id":revision,"reused":reused,
        "artifacts":{"fit":out.join("merged.fit"),"report":out.join("merge-report.json")},"next_action":null}),
    )
}
