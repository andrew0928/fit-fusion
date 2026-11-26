#[path = "../../effective_args.rs"]
mod effective_args;
#[path = "../../query_config.rs"]
mod query_config;
#[path = "../../query_filters.rs"]
mod query_filters;
#[path = "../../query_output.rs"]
mod query_output;
use query_filters::{Filters, parse_zone};
mod auth;
mod cache;
#[path = "../../cli_help.rs"]
mod cli_help;
mod client;
use auth::Credentials;
use cache::Original;
use clap::{FromArgMatches, Parser, Subcommand, ValueEnum, error::ErrorKind};
use client::{Client, Https, Transport, Window};
use fit_inspector::{activity::FIT_EPOCH_UNIX, inventory, write};
use jiff::{Timestamp, tz::TimeZone};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Debug, Serialize)]
pub struct Error {
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after: Option<String>,
    #[serde(skip)]
    exit: u8,
}
type Result<T> = std::result::Result<T, Error>;
impl Error {
    fn new(code: &'static str, exit: u8, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            exit,
            retry_after: None,
        }
    }
    fn auth(s: &str) -> Self {
        Self::new("garmin_auth", 4, s)
    }
    fn data(s: impl Into<String>) -> Self {
        Self::new("invalid_data", 3, s)
    }
    fn protocol() -> Self {
        Self::new(
            "garmin_response",
            6,
            "unexpected Garmin Personal API response",
        )
    }
    fn usage(s: impl Into<String>) -> Self {
        Self::new("usage", 2, s)
    }
}
impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::new(
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                "conflict"
            } else {
                "io_error"
            },
            3,
            e.to_string(),
        )
    }
}

#[derive(Parser)]
#[command(
    about = "Query Garmin Connect cycling activities and export original FIT files",
    arg_required_else_help = true,
    after_help = "Settings\n  config init creates ~/.fit-fusion/config.yaml. Explicit flags override YAML, then built-in defaults.\n  See --skill config for provenance on stderr.\n\nExamples\n  fit-query-garmin --env-file .env list --date 2026-11-21 --json\n  fit-query-garmin --env-file .env export --id 123456789 --out selected --json\n  fit-query-garmin --skill"
)]
struct Cli {
    /// Dotenv path. Overrides tools.fit-query-garmin.env_file (default .env in working directory).
    #[arg(long, global = true, value_name = "FILE", default_value = ".env")]
    env_file: PathBuf,
    /// Original FIT cache. Overrides tools.fit-query-garmin.cache_dir (default ~/Library/Caches/fit-query-garmin).
    #[arg(long, global = true, value_name = "DIR")]
    cache_dir: Option<PathBuf>,
    /// Date zone. Overrides common.timezone (default Asia/Taipei).
    #[arg(long,global=true,default_value="Asia/Taipei",value_parser=parse_zone)]
    timezone: TimeZone,
    #[command(flatten)]
    output: query_output::Output,
    /// Show offline agent instructions
    #[arg(long,num_args=0..=1,default_missing_value="index",value_name="TOPIC",conflicts_with_all=["json","text"])]
    skill: Option<Skill>,
    #[command(subcommand)]
    action: Option<Action>,
}
#[derive(Clone, Copy, ValueEnum)]
enum Skill {
    Index,
    Author,
    Config,
    List,
    Read,
    Export,
    Auth,
}
#[derive(Subcommand)]
enum Action {
    /// List cycling activities by local departure date
    #[command(
        after_help = "Example\n  fit-query-garmin --env-file .env list --from 2026-11-01 --until 2026-12-01 --json"
    )]
    List(Filters),
    /// Read one original's metadata and record-stream inventory
    #[command(
        after_help = "Example\n  fit-query-garmin --env-file .env read --id 123456789 --json"
    )]
    Read {
        /// Decimal Garmin activity ID from list
        #[arg(long,value_parser=id)]
        id: u64,
        /// Only use an already verified local original
        #[arg(long)]
        offline: bool,
    },
    /// Export a single original or a batch by date
    #[command(group(clap::ArgGroup::new("selection").args(["id","date","from","device"]).required(true).multiple(true)),
        after_help = "Example\n  fit-query-garmin --env-file .env export --date 2026-11-21 --out selected --json"
    )]
    Export {
        /// Select a single Garmin activity ID
        #[arg(long,value_parser=id,conflicts_with_all=["date","from","until","device"])]
        id: Option<u64>,
        #[command(flatten)]
        filters: Filters,
        /// Directory for original FIT copies
        #[arg(long, value_name = "DIR")]
        out: PathBuf,
        /// Use a cached original without contacting Garmin
        #[arg(long, requires = "id", conflicts_with_all=["date","from","until","device"])]
        offline: bool,
    },
}
fn id(s: &str) -> std::result::Result<u64, String> {
    s.parse::<u64>()
        .ok()
        .filter(|i| *i > 0 && i.to_string() == s)
        .ok_or_else(|| "expected a positive decimal Garmin activity ID".into())
}
impl Filters {
    fn window(&self, zone: TimeZone) -> Result<Option<Window>> {
        Ok(self
            .days()
            .map_err(Error::usage)?
            .map(|days| Window { days, zone }))
    }
}
fn recorder(fit: &Original) -> Value {
    let Some(device) = fit.metadata.device else {
        return Value::Null;
    };
    let (name, origin) = device.name(fit.metadata.product_name.as_ref());
    json!({"manufacturer":device.manufacturer,"product":device.product,"name":name,"name_source":origin})
}
fn read(id: u64, fit: &Original, name: Option<String>) -> Result<Value> {
    let (start, duration) = fit
        .metadata
        .cycling_session()?
        .ok_or_else(|| Error::data("expected a cycling activity"))?;
    let start = Timestamp::from_second(i64::from(start) + FIT_EPOCH_UNIX).expect("valid FIT date");
    let end = Timestamp::from_millisecond(start.as_millisecond() + i64::from(duration))
        .expect("valid FIT date");
    let streams = inventory::record_streams(&fit.bytes, "Garmin FIT")?;
    Ok(
        json!({"status":"ok","activity":{"id":id.to_string(),"source_id":"garmin_connect","device":recorder(fit), "activity_name":name,"device_name":fit.metadata.device_name(),
        "sport":"cycling","start_utc":start,"end_utc":end,"sha256":fit.hash,"path":fit.path,"cache_hit":fit.cache_hit},"streams":streams}),
    )
}
fn export_one(id: u64, fit: &Original, out: &Path, name: Option<&str>) -> Result<Value> {
    let path = out.join(format!("{id}.fit"));
    let status = write::copy_original(&fit.bytes, &path)?;
    Ok(
        json!({"source_id":"garmin_connect","id":id.to_string(),"sha256":fit.hash,"path":path,
        "format":"original","status":status,"cache_hit":fit.cache_hit,"activity_name":name,"device_name":fit.metadata.device_name()}),
    )
}
fn selected<T: Transport>(
    client: &mut Client<T>,
    cache: &Path,
    filters: &Filters,
    timezone: TimeZone,
) -> Result<Vec<client::Summary>> {
    let rows = client.list(filters.window(timezone)?.as_ref())?;
    let mut selected = Vec::new();
    for mut row in rows {
        // Listing stays usable with an empty or read-only cache.
        let _ = cache::remember_name(cache, &row.id, row.activity_name.as_deref());
        let fit = if let Some(device) = filters.device {
            let fit =
                cache::original(client, cache, row.id.parse().expect("normalized ID"), false)?;
            if fit.metadata.device != Some(device) {
                continue;
            }
            Some(fit)
        } else {
            cache::cached_original(cache, &row.id).ok().flatten()
        };
        if let Some(fit) = fit {
            row.device = Some(recorder(&fit));
            row.device_name = fit.metadata.device_name().or(row.device_name);
        }
        selected.push(row);
    }
    Ok(selected)
}
fn run<T: Transport>(
    client: &mut Client<T>,
    action: Action,
    cache: &Path,
    timezone: TimeZone,
) -> Result<(Value, u8)> {
    let reply = match action {
        Action::List(filters) => {
            json!({"status":"ok","activities":selected(client,cache,&filters,timezone)?})
        }
        Action::Read { id, offline } => read(
            id,
            &cache::original(client, cache, id, offline)?,
            cache::activity_name(cache, &id.to_string()),
        )?,
        Action::Export {
            id,
            filters,
            out,
            offline,
        } => {
            let ids = if let Some(id) = id {
                vec![(id, cache::activity_name(cache, &id.to_string()))]
            } else {
                if filters.date.is_none() && filters.from.is_none() && filters.device.is_none() {
                    return Err(Error::usage(
                        "export needs --id, --date, --from/--until or --device",
                    ));
                }
                client
                    .list(filters.window(timezone)?.as_ref())?
                    .into_iter()
                    .map(|a| {
                        let _ = cache::remember_name(cache, &a.id, a.activity_name.as_deref());
                        (a.id.parse().expect("normalized ID"), a.activity_name)
                    })
                    .collect()
            };
            let mut results = Vec::new();
            let mut exit = 0;
            let mut stop = false;
            let mut successes = 0;
            for (id, name) in ids {
                let path = out.join(format!("{id}.fit"));
                if stop {
                    results.push(json!({"id":id.to_string(),"path":path,"status":"not_attempted","source_id":"garmin_connect","activity_name":name,"device_name":null}));
                    continue;
                }
                let attempt = cache::original(client, cache, id, offline).and_then(|fit| {
                    if filters
                        .device
                        .is_some_and(|d| fit.metadata.device != Some(d))
                    {
                        return Ok(None);
                    }
                    export_one(id, &fit, &out, name.as_deref()).map(Some)
                });
                match attempt {
                    Ok(Some(item)) => {
                        results.push(item);
                        successes += 1;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        exit = exit.max(e.exit);
                        stop = e.exit >= 4;
                        results.push(json!({"id":id.to_string(),"source_id":"garmin_connect","path":path,"activity_name":name,"device_name":null,"status":if e.code=="conflict"{"conflict"}else{"error"},"error":e}));
                    }
                }
            }
            let status = if exit == 0 {
                "ok"
            } else if successes > 0 {
                "partial"
            } else {
                "error"
            };
            return Ok((json!({"status":status,"exports":results}), exit));
        }
    };
    Ok((reply, 0))
}
fn main() -> ExitCode {
    let (tree, matches, origins) = query_config::parse(cli_help::command::<Cli>(
        "Garmin Connect Personal API + FIT wire reader",
        "",
    ));
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    cli.output.validate().unwrap_or_else(|e| e.exit());
    let usage = |message: &str| -> ! {
        cli_help::command::<Cli>("Garmin Connect Personal API", "")
            .error(ErrorKind::InvalidValue, message)
            .exit()
    };
    if let Some(topic) = cli.skill {
        if cli.action.is_some() {
            usage("--skill must be used on its own");
        }
        print!(
            "{}",
            match topic {
                Skill::Index => include_str!("../guides/index.md"),
                Skill::Author => cli_help::AUTHOR_GUIDE,
                Skill::Config => query_config::GUIDE,
                Skill::List => include_str!("../guides/list.md"),
                Skill::Read => include_str!("../guides/read.md"),
                Skill::Export => include_str!("../guides/export.md"),
                Skill::Auth => include_str!("../guides/auth.md"),
            }
        );
        return ExitCode::SUCCESS;
    }
    let action = cli
        .action
        .unwrap_or_else(|| usage("expected list, read or export"));
    let cache = cli
        .cache_dir
        .unwrap_or_else(|| usage("provide --cache-dir when HOME is unset"));
    effective_args::report(&tree, &matches, &origins);
    let mut client = Client::new(
        Https::new(),
        Credentials {
            env_file: cli.env_file,
        },
    );
    let (result, exit) = match run(&mut client, action, &cache, cli.timezone.clone()) {
        Ok(result) => result,
        Err(e) if e.exit == 2 => usage(&e.message),
        Err(e) => (json!({"status":"error","error":e}), e.exit),
    };
    cli.output.print(&result, &cli.timezone);
    ExitCode::from(exit)
}
#[cfg(test)]
mod tests;
