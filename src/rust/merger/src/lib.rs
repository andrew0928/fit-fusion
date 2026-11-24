mod analysis;
pub mod config;
pub mod data_source;
mod native;
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Fit,
    FitAndParquet,
}
const VERSION: &str = "configured-fit-v3";
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
fn write_result(
    out: &Path,
    bytes: &[u8],
    report: &Value,
    parquet: Option<tempfile::NamedTempFile>,
) -> Result<bool> {
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let contents = serde_json::to_vec_pretty(report).map_err(std::io::Error::other)?;
    let with_parquet = parquet.is_some();
    let reuse = || -> Result<bool> {
        if fs::read(out.join("merged.fit")).ok().as_deref() == Some(bytes)
            && fs::read(out.join("merge-report.json")).ok().as_deref() == Some(contents.as_slice())
            && (!with_parquet
                || file_hash(&out.join("merged.parquet"))
                    .ok()
                    .as_deref()
                    .is_some_and(|digest| Some(digest) == report["parquet"]["sha256"].as_str()))
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
    if let Some(file) = parquet {
        file.persist(tmp.path().join("merged.parquet"))
            .map_err(|e| e.error)?;
    }
    use rustix::fs::{CWD, RenameFlags, renameat_with};
    match renameat_with(CWD, tmp.path(), CWD, out, RenameFlags::NOREPLACE) {
        Ok(()) => Ok(false),
        Err(rustix::io::Errno::EXIST) => reuse(),
        Err(e) => Err(std::io::Error::from(e).into()),
    }
}
pub fn merge_files(inputs: [PathBuf; 2], out: PathBuf) -> Result<Value> {
    merge_files_with_format(inputs, out, ExportFormat::Fit)
}
pub fn merge_files_with_format(
    inputs: [PathBuf; 2],
    out: PathBuf,
    format: ExportFormat,
) -> Result<Value> {
    let [first, second] = inputs;
    manual_pair(
        &config::Settings::embedded()?,
        FitDirectory::open_file(&first)?,
        FitDirectory::open_file(&second)?,
        &out,
        format,
    )
}

fn file_hash(path: &Path) -> std::io::Result<String> {
    let mut digest = Sha256::new();
    std::io::copy(&mut fs::File::open(path)?, &mut digest)?;
    Ok(hex::encode(digest.finalize()))
}

/// Merge two already-open recordings with the production dual-file defaults.
/// A directory caller can pass DataSource::open_activity results without opening paths again.
pub fn merge_recordings(
    first: ActivityRecording,
    second: ActivityRecording,
    out: &Path,
) -> Result<Value> {
    manual_pair(
        &config::Settings::embedded()?,
        first,
        second,
        out,
        ExportFormat::Fit,
    )
}
fn manual_pair(
    settings: &config::Settings,
    first: ActivityRecording,
    second: ActivityRecording,
    out: &Path,
    format: ExportFormat,
) -> Result<Value> {
    let primary_id = settings.primary_source().id().to_owned();
    let device = settings
        .device(&primary_id)
        .ok_or_else(|| Error::usage("manual FIT pairing requires a primary device identity"))?;
    let (b, g) = match (
        first.metadata.device == device,
        second.metadata.device == device,
    ) {
        (true, false) => (first, second),
        (false, true) => (second, first),
        (true, true) => {
            return Err(Error::usage(
                "both files match the primary model; use date mode with separate source directories",
            ));
        }
        _ => {
            return Err(Error::data(
                "neither FIT matches the configured primary device",
            ));
        }
    };
    let mut rides = BTreeMap::from([(primary_id.clone(), Ride::from_recording(b)?)]);
    let ids = workflow::required(settings);
    if ids.len() > 1 {
        return Err(Error::usage(
            "use date mode to identify multiple configured secondary sources",
        ));
    }
    if let Some(id) = ids.first() {
        if settings.device(id) != Some(g.metadata.device) {
            return Err(Error::usage(
                "second input must match the configured secondary device",
            ));
        }
        rides.insert(id.clone(), Ride::from_recording(g)?);
    } else if format == ExportFormat::FitAndParquet {
        // A manually supplied FIT is part of the analysis input even when every stream selects the primary.
        let mut configured = settings
            .sources()
            .filter(|s| settings.device(s.id()) == Some(g.metadata.device));
        let id = match (configured.next(), configured.next()) {
            (Some(s), None) => s.id(),
            _ => "@input-secondary",
        };
        rides.insert(id.to_owned(), Ride::from_recording(g)?);
    }
    export(
        settings,
        &MatchedRide { primary_id, rides },
        out,
        false,
        format,
    )
}
fn export(
    settings: &config::Settings,
    pair: &MatchedRide,
    out: &Path,
    revision_path: bool,
    format: ExportFormat,
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
    let mut calories=selected.calories.as_ref().map(|c|json!({
        "source":selected.calorie_source.and_then(|r|pair.rides.iter().find(|(_,v)|std::ptr::eq(*v,r)).map(|(id,_)|id)),
        "bryton_original":data_source::number(&b.session,11),"total_calories":c.total,"metabolic_calories":c.metabolic,
        "active_calories":c.active(),"developer_index":selected.write_total.then_some(c.dev),
        "application_id":hex::encode(output::APP_ID)
    }));
    if let Some(c) = &mut calories {
        c["primary_original"] = json!(data_source::number(&b.session, 11));
        if b.device.manufacturer != data_source::Device::RIDER_460.manufacturer {
            c.as_object_mut().unwrap().remove("bryton_original");
        }
    }
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
        .map(|f| (&f.name, &f.ranges))
        .collect();
    let first_overlap = selected.overlaps.values().next();
    let mut report = json!({"schema_version":1,"rules_version":VERSION,"inputs":inputs,"primary_timeline":pair.primary_id,
        "selection":settings.selection(),"unlisted_streams":"preserve_primary","overlap":{"record_ratios":first_overlap.map(|r|r[0]),"session_ratios":first_overlap.map(|r|r[1]),"by_source":selected.overlaps},
        "heart_rate":s.report,"heart_rate_sources":hr_ranges,"stream_sources":field_sources,"calories":calories,"output_sha256":hash(&bytes)});
    let hashes: BTreeMap<_, _> = pair.rides.iter().map(|(id, r)| (id, &r.hash)).collect();
    let revision = match format {
        ExportFormat::Fit => serde_json::to_vec(&(VERSION, &hashes, settings.selection())),
        ExportFormat::FitAndParquet => serde_json::to_vec(&(
            VERSION,
            &hashes,
            settings.selection(),
            "parquet",
            analysis::SCHEMA_VERSION,
        )),
    }
    .map(|identity| hash(&identity))
    .map_err(std::io::Error::other)?;
    let out = if revision_path {
        out.join(&b.hash).join(&revision)
    } else {
        out.to_owned()
    };
    let mut artifacts =
        json!({"fit":out.join("merged.fit"),"report":out.join("merge-report.json")});
    let parquet = if format == ExportFormat::FitAndParquet {
        let parent = out
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        analysis::write(file.as_file_mut(), pair, &selected, settings, &report)?;
        file.as_file().sync_all()?;
        report["parquet"] =
            json!({"schema_version":analysis::SCHEMA_VERSION,"sha256":file_hash(file.path())?});
        artifacts["parquet"] = json!(out.join("merged.parquet"));
        Some(file)
    } else {
        None
    };
    let reused = write_result(&out, &bytes, &report, parquet)?;
    Ok(
        json!({"schema_version":1,"status":"exported","code":"ok","revision_id":revision,"reused":reused,
        "artifacts":artifacts,"next_action":null}),
    )
}
