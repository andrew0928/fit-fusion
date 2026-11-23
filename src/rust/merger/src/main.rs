mod output;
mod quality;
mod ride;
use output::Calories;
use quality::Selection;
use ride::Ride;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::ExitCode,
};

const VERSION: &str = "offline-dual-fit-v1";
const HELP: &str = "fit-fusion merge INPUT.fit INPUT.fit --out DIRECTORY [--json]
Merge one Bryton Rider 460 and one Garmin CIRQA cycling recording offline.
Inputs may be supplied in either order. Creates merged.fit and merge-report.json.
The default stream rules are embedded. No account or network is required.
Identical results can reuse a directory. Different contents require a new directory.
fit-fusion --skill <index|merge|config|publish>";
pub struct Error {
    code: &'static str,
    message: String,
    details: Value,
    exit: u8,
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
    fn usage(s: impl Into<String>) -> Self {
        Self::new("usage", s, 2)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new("io_error", e.to_string(), 3)
    }
}
type Result<T> = std::result::Result<T, Error>;
fn hash(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}
fn overlap((a, b): (u32, u32), (c, d): (u32, u32)) -> [f64; 2] {
    let span = b.min(d).saturating_sub(a.max(c)) as f64;
    [span / (b - a) as f64, span / (d - c) as f64]
}
fn session_span(r: &Ride) -> Result<(u32, u32)> {
    let a =
        ride::number(&r.session, 2).ok_or_else(|| Error::quality("session start unavailable"))?;
    let ms = ride::number(&r.session, 7)
        .ok_or_else(|| Error::quality("session elapsed time unavailable"))?;
    let end = a
        .checked_add(ms.div_ceil(1000))
        .filter(|&end| end > a)
        .ok_or_else(|| Error::quality("invalid session duration"))?;
    Ok((a, end))
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
fn merge(inputs: Vec<PathBuf>, out: PathBuf) -> Result<Value> {
    let a = Ride::read(inputs[0].clone())?;
    let b = Ride::read(inputs[1].clone())?;
    let (b, g) = match (a.device, b.device) {
        ("bryton", "garmin") => (a, b),
        ("garmin", "bryton") => (b, a),
        _ => return Err(Error::data("expected one Rider 460 and one CIRQA")),
    };
    let defaults: Value =
        serde_json::from_str(include_str!("../../../../docs/config/config.example.json"))
            .expect("embedded config");
    let threshold = defaults["merge"]["min_overlap_ratio"].as_f64().unwrap();
    let overlap_records = overlap(b.span(), g.span());
    let overlap_sessions = overlap(session_span(&b)?, session_span(&g)?);
    if overlap_records
        .iter()
        .chain(&overlap_sessions)
        .any(|&r| r < threshold)
    {
        let mut e = Error::new(
            "unmatched_activities",
            "riding intervals do not meet the mutual overlap threshold",
            3,
        );
        e.details =
            json!({"records":overlap_records,"sessions":overlap_sessions,"minimum":threshold});
        return Err(e);
    }
    let s = quality::select(&b, &g)?;
    let c = Calories::read(&b, &g)?;
    let bytes = output::build(&b, &g, &s, &c)?;
    let source = |r: &Ride| json!({"path":r.path,"sha256":r.hash,"record_interval_fit_seconds":r.span(),"unknown_native_message_field_pairs":r.unknown_fields});
    let report = json!({"schema_version":1,"rules_version":VERSION,"inputs":{"bryton":source(&b),"garmin":source(&g)},
        "selection":defaults["merge"],"overlap":{"record_ratios":overlap_records,"session_ratios":overlap_sessions},
        "heart_rate":s.report,"heart_rate_sources":provenance(&s),"primary_timeline":"bryton",
        "calories":{"source":"garmin","bryton_original":ride::number(&b.session,11),"total_calories":c.total,"metabolic_calories":c.metabolic,
        "active_calories":c.metabolic.and_then(|v|c.total.checked_sub(v)),"developer_index":c.dev,"application_id":hex::encode(output::APP_ID)},
        "output_sha256":hash(&bytes)});
    let reused = write_result(&out, &bytes, &report)?;
    Ok(
        json!({"schema_version":1,"status":"exported","code":"ok","reused":reused,"artifacts":{"fit":out.join("merged.fit"),
        "report":out.join("merge-report.json")},"next_action":null}),
    )
}
fn run(args: &[String]) -> Result<()> {
    let a: Vec<_> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["--help" | "-h"] | ["merge", "--help" | "-h"] => {
            println!("{HELP}");
            return Ok(());
        }
        ["--skill", topic] => {
            println!(
                "{}",
                match *topic {
                    "index" => HELP,
                    "merge" =>
                        "fit-fusion merge bryton.fit garmin.fit --out rides/merged --json\nRead stdout as one JSON. Exit 0: inspect artifacts.fit and artifacts.report. Exit 2: fix usage. Exit 3: inspect code and details; quality_review requires review before publishing. Identical inputs/rules may reuse a verified result. No upload occurs.",
                    "config" =>
                        "Offline dual-file mode uses the embedded v1 default priorities (Bryton timeline; Garmin heart-rate fallback and calories). Supply both files and --out. User config, date discovery and API credentials are planned steps.",
                    "publish" =>
                        "This release exports locally. Upload artifacts.fit manually if desired. Automated Strava publishing is a later M2 step.",
                    _ =>
                        return Err(Error::usage(
                            "available skills: index, merge, config, publish"
                        )),
                }
            );
            return Ok(());
        }
        _ => {}
    }
    if a.first() != Some(&"merge") {
        return Err(Error::usage(HELP));
    }
    let mut args = a[1..].iter().copied();
    let mut out = None;
    let mut inputs = vec![];
    let mut as_json = false;
    while let Some(x) = args.next() {
        match x {
            "--json" if !as_json => as_json = true,
            "--out" if out.is_none() => {
                out = Some(PathBuf::from(
                    args.next()
                        .filter(|s| !s.starts_with('-') && !s.is_empty())
                        .ok_or_else(|| Error::usage("missing --out directory"))?,
                ))
            }
            x if !x.starts_with('-') => inputs.push(PathBuf::from(x)),
            _ => return Err(Error::usage(format!("unsupported or duplicate option {x}"))),
        }
    }
    if inputs.len() != 2 {
        return Err(Error::usage("provide two input FIT files"));
    }
    let result = merge(
        inputs,
        out.ok_or_else(|| Error::usage("--out is required"))?,
    )?;
    if as_json {
        println!("{result}");
    } else {
        println!(
            "FIT: {}\nReport: {}",
            result["artifacts"]["fit"].as_str().unwrap(),
            result["artifacts"]["report"].as_str().unwrap()
        );
    }
    Ok(())
}
fn main() -> ExitCode {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fit-fusion: {}", e.message);
            if args.iter().any(|a| a == "--json") {
                println!(
                    "{}",
                    json!({"schema_version":1,"status":"error","code":e.code,"message":e.message,"details":e.details,"artifacts":{},"next_action":null})
                );
            }
            ExitCode::from(e.exit)
        }
    }
}
