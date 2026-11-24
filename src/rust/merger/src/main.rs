use fit_fusion::{Error, Result, config::Settings, data_source::ActivityQuery, merge_files};
use serde_json::json;
use std::{path::PathBuf, process::ExitCode};
const HELP: &str = "fit-fusion merge INPUT.fit INPUT.fit --out DIRECTORY [--json]
Merge one Bryton Rider 460 and one Garmin CIRQA cycling recording offline.
Inputs may be supplied in either order. Creates merged.fit and merge-report.json.
The default stream rules are embedded. No account or network is required.
Identical results can reuse a directory. Different contents require a new directory.
fit-fusion [--config FILE] config check [--json]
fit-fusion [--config FILE] activities list --source ID [--json]
fit-fusion --skill <index|merge|config|sources|publish>";
fn run(args: &[String]) -> Result<()> {
    let a: Vec<_> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["--help" | "-h"]
        | ["merge" | "config" | "activities", "--help" | "-h"]
        | ["config", "check", "--help"]
        | ["activities", "list", "--help"] => {
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
                        "fit-fusion --config FILE config check --json\nValidates the v1 config offline. Inspect sources, primary_source, streams and destinations. Exit 2: fix the setting at details.path. Credentials are omitted. Explicit dual-file merge uses embedded defaults.",
                    "sources" =>
                        "fit-fusion --config FILE activities list --source ID --json\nRecursively list local cycling FITs for the configured device. IDs are SHA-256 of the original bytes. Timestamps are Unix UTC seconds. Empty list is success. Missing directory, invalid FIT or an unavailable reader returns exit 3. Garmin Connect and Apple Health ZIP readers are planned.",
                    "publish" =>
                        "This release exports locally. Upload artifacts.fit manually if desired. Automated Strava publishing is a later M2 step.",
                    _ =>
                        return Err(Error::usage(
                            "available skills: index, merge, config, sources, publish"
                        )),
                }
            );
            return Ok(());
        }
        _ => {}
    }
    let (config_path, command) = match a.as_slice() {
        ["--config", path, rest @ ..] if !path.starts_with('-') => {
            (Some(PathBuf::from(path)), rest)
        }
        other => (None, other),
    };
    if matches!(command.first(), Some(&"config" | &"activities")) {
        return configured(command, config_path);
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
    let result = merge_files(
        inputs.try_into().expect("two inputs checked"),
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
fn configured(args: &[&str], path: Option<PathBuf>) -> Result<()> {
    let (args, json_output) = match args.strip_suffix(&["--json"]) {
        Some(rest) => (rest, true),
        None => (args, false),
    };
    if matches!(
        args,
        ["config", "check", "--help"] | ["activities", "list", "--help"]
    ) {
        println!("{HELP}");
        return Ok(());
    }
    let source = match args {
        ["config", "check"] => None,
        ["activities", "list", "--source", id] => Some(*id),
        _ => return Err(Error::usage(HELP)),
    };
    let path = path
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|d| PathBuf::from(d).join(".config/fit-fusion/config.json"))
        })
        .ok_or_else(|| Error::usage("provide --config FILE"))?;
    let settings = Settings::load(&path)?;
    let result = if let Some(id) = source {
        let source = settings
            .source(id)
            .ok_or_else(|| Error::usage(format!("unknown source: {id}")))?;
        let activities=source.list_activities(&ActivityQuery::All)?.into_iter().map(|a|json!({
            "id":a.activity_id,"source_id":a.source_id,"start_utc":a.start_utc,"end_utc":a.end_utc
        })).collect::<Vec<_>>();
        json!({"schema_version":1,"status":"ok","source":id,"activities":activities})
    } else {
        let streams = settings
            .selected_fields()
            .map(|field| {
                (
                    field.to_owned(),
                    json!(
                        settings
                            .sources_for(field)
                            .map(|s| s.id())
                            .collect::<Vec<_>>()
                    ),
                )
            })
            .collect::<serde_json::Map<_, _>>();
        json!({"schema_version":1,"status":"ok","primary_source":settings.primary_source().id(),
            "sources":settings.sources().map(|s|json!({"id":s.id(),"kind":s.kind().name()})).collect::<Vec<_>>(),
            "destinations":settings.targets().map(|t|json!({"id":t.id,"kind":t.kind})).collect::<Vec<_>>(),"streams":streams})
    };
    if json_output {
        println!("{result}");
    } else {
        println!("{result:#}");
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
