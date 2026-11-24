use fit_fusion::{Error, Result, config::Settings, data_source::ActivityQuery, merge_files};
use serde_json::json;
use std::{path::PathBuf, process::ExitCode};
const HELP: &str = "fit-fusion [--config FILE] merge --date YYYY-MM-DD [--json]
Find cycling activities starting that day in the configured time zone. Write each verified FIT and report under output_dir.
fit-fusion [--config FILE] merge PRIMARY.fit (--garmin auto | --garmin-id ID) --out DIRECTORY [--json]
Local directories stay offline. Garmin Connect reads activity lists and downloads originals, then caches verified FITs. No uploads.
fit-fusion [--config FILE] merge INPUT.fit INPUT.fit --out DIRECTORY [--json]
Merge one Bryton Rider 460 and one Garmin CIRQA cycling recording offline.
Inputs may be supplied in either order. Creates merged.fit and merge-report.json.
The default stream rules are embedded. No account or network is required.
Identical results can reuse a directory. Different contents require a new directory.
fit-fusion [--config FILE] config init|check [--json]
fit-fusion [--config FILE] activities list --source ID [--json]
fit-fusion --skill <index|merge|config|sources|publish>";
fn run(args: &[String]) -> Result<u8> {
    let a: Vec<_> = args.iter().map(String::as_str).collect();
    match a.as_slice() {
        ["--help" | "-h"]
        | ["merge" | "config" | "activities", "--help" | "-h"]
        | ["config", "init" | "check", "--help"]
        | ["activities", "list", "--help"] => {
            println!("{HELP}");
            return Ok(0);
        }
        ["--skill", topic] => {
            println!(
                "{}",
                match *topic {
                    "index" => HELP,
                    "merge" =>
                        "fit-fusion --config FILE merge --date 2026-11-21 --json\nInspect rides[] for artifacts, reused, code and details. Partial batches preserve successful rides. For ambiguous_match use merge PRIMARY.fit --garmin-id ID --out NEW_DIR. Retry rate limits after retry_after. Exit 4: update Garmin key. Exit 5: wait. Exit 6: check Garmin connection.\nfit-fusion merge bryton.fit garmin.fit --out rides/merged --json\nRead stdout as one JSON. Exit 0: inspect artifacts.fit and artifacts.report. Exit 2: fix usage. Exit 3: inspect code and details; quality_review requires review before publishing. Identical inputs/rules may reuse a verified result. No upload occurs.",
                    "config" =>
                        "fit-fusion --config FILE config init --json\nWrites the credential-free template or preserves the existing config. Set your source paths and Garmin key, then run fit-fusion --config FILE config check --json.\nValidates the v1 config offline. Inspect sources, primary_source, streams and destinations. Exit 2: fix the setting at details.path. Credentials are omitted. Explicit dual-file merge uses embedded defaults unless --config is supplied.",
                    "sources" =>
                        "fit-fusion --config FILE activities list --source ID --json\nRecursively list local cycling FITs for the configured device. IDs are SHA-256 of the original bytes. Timestamps are Unix UTC seconds. Empty list is success. Missing directory, invalid FIT or an unavailable reader returns exit 3. Garmin Connect lists account cycling activities, verifies downloaded devices at merge time and reuses cached originals. Apple Health ZIP is reserved for analysis.",
                    "publish" =>
                        "This release exports locally. Upload artifacts.fit manually if desired. Automated Strava publishing is a later M2 step.",
                    _ =>
                        return Err(Error::usage(
                            "available skills: index, merge, config, sources, publish"
                        )),
                }
            );
            return Ok(0);
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
        return configured(command, config_path).map(|_| 0);
    }
    if command == ["merge", "--help"] {
        println!("{HELP}");
        return Ok(0);
    }
    if command.first() != Some(&"merge") {
        return Err(Error::usage(HELP));
    }
    let mut args = command[1..].iter().copied();
    let mut out = None;
    let (mut date, mut garmin, mut known) = (None, None, None);
    let mut inputs = vec![];
    let mut as_json = false;
    while let Some(x) = args.next() {
        match x {
            "--date" | "--garmin" | "--garmin-id" => {
                let value = args
                    .next()
                    .filter(|v| !v.starts_with('-') && !v.is_empty())
                    .ok_or_else(|| Error::usage(format!("missing value for {x}")))?;
                let slot = match x {
                    "--date" => &mut date,
                    "--garmin" => &mut garmin,
                    _ => &mut known,
                };
                if slot.replace(value).is_some() {
                    return Err(Error::usage(format!("duplicate {x}")));
                }
            }
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
    if garmin.is_some_and(|g| g != "auto") || garmin.is_some() && known.is_some() {
        return Err(Error::usage("use --garmin auto or --garmin-id ID"));
    }
    let (result, exit) = if let Some(day) = date {
        if !inputs.is_empty() || out.is_some() || known.is_some() || garmin.is_some() {
            return Err(Error::usage(
                "date mode uses configured sources and output_dir",
            ));
        }
        fit_fusion::workflow::date(&Settings::load(&settings_path(config_path)?)?, day)?
    } else {
        let out = out.ok_or_else(|| Error::usage("--out is required"))?;
        if inputs.len() == 2 && config_path.is_none() && garmin.is_none() && known.is_none() {
            (merge_files(inputs.try_into().unwrap(), out)?, 0)
        } else {
            let s = Settings::load(&settings_path(config_path)?)?;
            (
                fit_fusion::workflow::manual(&s, &inputs, garmin.is_some(), known, &out)?,
                0,
            )
        }
    };
    if as_json {
        println!("{result}");
    } else {
        println!("{result:#}");
    }
    Ok(exit)
}
fn settings_path(path: Option<PathBuf>) -> Result<PathBuf> {
    path.or_else(|| std::env::home_dir().map(|p| p.join(".config/fit-fusion/config.json")))
        .ok_or_else(|| Error::usage("provide --config FILE"))
}
fn configured(args: &[&str], path: Option<PathBuf>) -> Result<()> {
    let (args, json_output) = match args.strip_suffix(&["--json"]) {
        Some(rest) => (rest, true),
        None => (args, false),
    };
    if matches!(
        args,
        ["config", "init" | "check", "--help"] | ["activities", "list", "--help"]
    ) {
        println!("{HELP}");
        return Ok(());
    }
    if args == ["config", "init"] {
        let path = settings_path(path)?;
        let created = Settings::init(&path)?;
        let result = json!({"schema_version":1,"status":"ok","code":if created {"created"}else{"already_exists"},
            "artifacts":{"config":path},"next_action":"config check"});
        if json_output {
            println!("{result}");
        } else {
            println!("{result:#}");
        }
        return Ok(());
    }
    let source = match args {
        ["config", "check"] => None,
        ["activities", "list", "--source", id] => Some(*id),
        _ => return Err(Error::usage(HELP)),
    };
    let settings = Settings::load(&settings_path(path)?)?;
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
        Ok(exit) => ExitCode::from(exit),
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
