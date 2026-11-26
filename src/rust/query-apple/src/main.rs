#[path = "../../cli_help.rs"]
mod cli_help;
#[path = "../../query_dates.rs"]
mod dates;
#[path = "../../effective_args.rs"]
mod effective_args;
mod fit;
mod health;
#[path = "../../query_config.rs"]
mod query_config;
#[path = "../../query_output.rs"]
mod query_output;
mod route;
use clap::{Args, FromArgMatches, Parser, Subcommand, ValueEnum, error::ErrorKind};
use jiff::{civil::Date, tz::TimeZone};
use serde_json::{Value, json};
use std::{
    error::Error,
    io,
    path::{Path, PathBuf},
    process::ExitCode,
};
type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn bad(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}

#[derive(Parser)]
#[command(
    about = "Query a local Apple Health export and convert cycling workouts to FIT",
    arg_required_else_help = true,
    after_help = "Settings\n  config init creates ~/.fit-fusion/config.yaml. Explicit flags override YAML, then built-in defaults.\n  See --skill config for provenance on stderr.\n\nExamples\n  fit-query-apple --archive export.zip list --date 2026-11-21 --text\n  fit-query-apple --skill export"
)]
struct Cli {
    /// Apple Health export ZIP. Required via --archive or tools.fit-query-apple.archive.
    #[arg(long, global = true, value_name = "ZIP")]
    archive: Option<PathBuf>,
    /// Date zone. Overrides common.timezone (default Asia/Taipei).
    #[arg(long,global=true,default_value="Asia/Taipei",value_parser=dates::parse_zone)]
    timezone: TimeZone,
    #[command(flatten)]
    output: query_output::Output,
    /// Read an offline agent guide
    #[arg(long,num_args=0..=1,default_missing_value="index",conflicts_with_all=["archive","json","text"])]
    skill: Option<Skill>,
    #[command(subcommand)]
    action: Option<Action>,
}
#[derive(Clone, Copy, ValueEnum)]
enum Skill {
    Index,
    Config,
    List,
    Read,
    Export,
}
#[derive(Args, Default)]
struct Select {
    /// Local departure day (YYYY-MM-DD)
    #[arg(long,value_parser=dates::parse_date,conflicts_with_all=["from","until"])]
    date: Option<Date>,
    /// First departure day (included)
    #[arg(long,requires="until",value_parser=dates::parse_date)]
    from: Option<Date>,
    /// End of departure-day range (excluded)
    #[arg(long,requires="from",value_parser=dates::parse_date)]
    until: Option<Date>,
    /// Exact Workout sourceName from list
    #[arg(long, value_name = "NAME")]
    source_name: Option<String>,
}
fn id(s: &str) -> std::result::Result<String, String> {
    if s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(s.to_ascii_lowercase())
    } else {
        Err("expected a 64-digit Workout ID from list".into())
    }
}
#[derive(Subcommand)]
enum Action {
    /// List cycling workouts, including overlapping recordings from different sources
    #[command(
        after_help = "Example\n  fit-query-apple --archive export.zip list --source-name MyWatch --text"
    )]
    List(Select),
    /// Read workout metadata, source sample ranges and route coverage
    #[command(
        after_help = "Example\n  fit-query-apple --archive export.zip read --id WORKOUT_ID --json"
    )]
    Read {
        #[arg(long,value_parser=id)]
        id: String,
    },
    /// Export one converted FIT per selected Workout
    #[command(
        after_help = "Example\n  fit-query-apple --archive export.zip export --date 2026-11-21 --out rides --json"
    )]
    Export {
        #[arg(long,value_parser=id,conflicts_with_all=["date","from","until","source_name"])]
        id: Option<String>,
        #[command(flatten)]
        filters: Select,
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
    },
}
fn usage(s: &str) -> ! {
    cli_help::command::<Cli>("Apple Health XML/GPX and FIT writer", "")
        .error(ErrorKind::InvalidValue, s)
        .exit()
}

fn run(file: &Path, action: &Action, zone: &TimeZone) -> Result<Value> {
    let empty = Select::default();
    let (filters, id) = match action {
        Action::List(f) => (f, None),
        Action::Read { id } => (&empty, Some(id)),
        Action::Export { filters, id, .. } => (filters, id.as_ref()),
    };
    let days = dates::days(filters.date, filters.from, filters.until).unwrap_or_else(|s| usage(&s));
    let (mut archive, xml) = health::open(file)?;
    let workouts: Vec<_> = health::list(&mut archive, &xml)?
        .into_iter()
        .filter(|w| {
            id.is_none_or(|id| *id == w.id)
                && filters
                    .source_name
                    .as_ref()
                    .is_none_or(|s| s == &w.source_name)
                && days.as_ref().is_none_or(|days| {
                    days.contains(&w.start_utc.to_zoned_datetime(zone.clone()).date())
                })
        })
        .collect();
    if id.is_some() && workouts.is_empty() {
        return Err(bad("Workout ID not found in this archive"));
    }
    if matches!(action, Action::List(_)) {
        return Ok(json!({"status":"ok","activities":workouts,"errors":[]}));
    }
    let observations = health::samples(&mut archive, &xml, &workouts)?;
    let mut exports = vec![];
    let mut errors = vec![];
    for (w, obs) in workouts.iter().zip(observations) {
        let mut missing = vec![];
        if obs.route_points == 0 {
            missing.push("route");
        }
        if !obs
            .rows
            .values()
            .flatten()
            .any(|p| p.contains_key(&health::Metric::Heart))
        {
            missing.push("heart_rate");
        }
        let info = json!({"missing":missing,"route_points":obs.route_points,"missing_route_files":obs.missing_routes});
        let mut destination = None;
        let result = (|| -> Result<Value> {
            if let Some(err) = &obs.error {
                return Err(bad(err));
            }
            let bytes = fit::encode(w, &obs)?;
            let streams =
                fit_inspector::inventory::record_streams(&bytes, "converted Apple Workout")?;
            let sha = health::hash(&bytes);
            match action {
                Action::Read { .. } => Ok(
                    json!({"status":"ok","activity":w,"format":"converted","sha256":sha,"streams":streams,"coverage":info}),
                ),
                Action::Export { out, .. } => {
                    let path = out.join(format!("{}-{sha}.fit", w.id));
                    destination = Some(path.clone());
                    let status = fit_inspector::write::copy_original(&bytes, &path)?;
                    Ok(
                        json!({"id":w.id,"source_id":w.source_id,"source_name":w.source_name,"activity_name":w.activity_name,"device_name":w.device_name,"format":"converted","path":path,"sha256":sha,"status":status,"coverage":info}),
                    )
                }
                _ => unreachable!(),
            }
        })();
        match (action, result) {
            (Action::Read { .. }, result) => return result,
            (_, Ok(value)) => exports.push(value),
            (_, Err(e)) => {
                let status = if e
                    .downcast_ref::<io::Error>()
                    .is_some_and(|e| e.kind() == io::ErrorKind::AlreadyExists)
                {
                    "conflict"
                } else {
                    "error"
                };
                let err = json!({"id":w.id,"path":destination,"message":e.to_string()});
                exports.push(json!({"id":w.id,"source_name":w.source_name,"source_id":w.source_id,"activity_name":w.activity_name,"device_name":w.device_name,"path":destination,"status":status,"error":err}));
                errors.push(err);
            }
        }
    }
    let status = if errors.is_empty() {
        "ok"
    } else if errors.len() == exports.len() {
        "error"
    } else {
        "partial"
    };
    Ok(json!({"status":status,"exports":exports,"errors":errors}))
}
fn main() -> ExitCode {
    let (tree, matches, origins) = query_config::parse(cli_help::command::<Cli>(
        "Apple Health XML/GPX and FIT writer",
        "",
    ));
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    cli.output.validate().unwrap_or_else(|e| e.exit());
    if let Some(topic) = cli.skill {
        if cli.action.is_some() {
            usage("--skill must be used on its own");
        }
        print!(
            "{}",
            match topic {
                Skill::Index => include_str!("../guides/index.md"),
                Skill::Config => query_config::GUIDE,
                Skill::List => include_str!("../guides/list.md"),
                Skill::Read => include_str!("../guides/read.md"),
                Skill::Export => include_str!("../guides/export.md"),
            }
        );
        return ExitCode::SUCCESS;
    }
    let file = cli
        .archive
        .unwrap_or_else(|| usage("provide --archive ZIP or set tools.fit-query-apple.archive in ~/.fit-fusion/config.yaml"));
    let action = cli
        .action
        .unwrap_or_else(|| usage("expected list, read or export"));
    effective_args::report(&tree, &matches, &origins);
    let result = run(&file, &action, &cli.timezone).unwrap_or_else(
        |e| json!({"status":"error","errors":[{"path":file,"message":e.to_string()}]}),
    );
    cli.output.print(&result, &cli.timezone);
    if result["status"] == "ok" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    }
}
