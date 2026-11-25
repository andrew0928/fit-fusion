mod cli;
#[path = "../../cli_help.rs"]
mod cli_help;
mod guidance;

use clap::{
    FromArgMatches,
    error::{ContextKind, ContextValue, ErrorKind},
};
use cli::{Action, Activities, Cli, Config, Merge};
use fit_fusion::{
    Error, ExportFormat, Result, config::Settings, data_source::ActivityQuery,
    merge_files_with_format, workflow,
};
use serde_json::{Value, json};
use std::{ffi::OsStr, path::PathBuf, process::ExitCode};

fn settings_path(path: Option<PathBuf>) -> Result<PathBuf> {
    path.or_else(|| std::env::home_dir().map(|p| p.join(".config/fit-fusion/config.json")))
        .ok_or_else(|| Error::usage("provide --config FILE"))
}

fn list(settings: &Settings, id: &str) -> Result<Value> {
    let source = settings
        .source(id)
        .ok_or_else(|| Error::usage(format!("unknown source: {id}")))?;
    let activities = source
        .list_activities(&ActivityQuery::All)?
        .into_iter()
        .map(|row| {
            json!({
                "id": row.activity_id, "source_id": row.source_id,
                "start_utc": row.start_utc, "end_utc": row.end_utc,
                "device": row.device
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({"schema_version":1,"status":"ok","source":id,"activities":activities}))
}

fn check(settings: &Settings) -> Value {
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
}

fn merge(args: Merge, path: Option<PathBuf>) -> Result<(Value, u8)> {
    let format = if args.parquet {
        ExportFormat::FitAndParquet
    } else {
        ExportFormat::Fit
    };
    if let Some(date) = args.date {
        return workflow::date_with_format(
            &Settings::load(&settings_path(path)?)?,
            &date.to_string(),
            format,
        );
    }
    let out = args.out.expect("clap requires --out in file mode");
    let first = args
        .first
        .expect("clap requires a primary FIT in file mode");
    let inputs = match args.second {
        Some(second) if path.is_none() => {
            return Ok((merge_files_with_format([first, second], out, format)?, 0));
        }
        Some(second) => vec![first, second],
        None => vec![first],
    };
    let settings = Settings::load(&settings_path(path)?)?;
    Ok((
        workflow::manual_with_format(
            &settings,
            &inputs,
            args.auto_match,
            args.garmin_id.as_deref(),
            &out,
            format,
        )?,
        0,
    ))
}

fn run(cli: Cli, command: &mut clap::Command) -> Result<u8> {
    if let Some(topic) = cli.skill {
        if cli.command.is_some() {
            return Err(Error::usage("use --skill separately from a subcommand"));
        }
        print!("{}", topic.page());
        return Ok(0);
    }
    let (result, exit) = match cli.command {
        Some(Action::Config { command: None }) | Some(Action::Activities { command: None }) => {
            let group = if matches!(cli.command, Some(Action::Config { .. })) {
                "config"
            } else {
                "activities"
            };
            if cli.json {
                return Err(Error::usage(format!(
                    "choose a {group} subcommand. See {group} --help"
                )));
            }
            print!(
                "{}",
                command.find_subcommand_mut(group).unwrap().render_help()
            );
            return Ok(0);
        }
        Some(Action::Config {
            command: Some(Config::Init),
        }) => {
            let path = settings_path(cli.config)?;
            let created = Settings::init(&path)?;
            (
                json!({"schema_version":1,"status":"ok","code":if created {"created"} else {"already_exists"},
                    "artifacts":{"config":path},"next_action":"config check"}),
                0,
            )
        }
        Some(Action::Config {
            command: Some(Config::Check),
        }) => (check(&Settings::load(&settings_path(cli.config)?)?), 0),
        Some(Action::Activities {
            command: Some(Activities::List { source }),
        }) => (
            list(&Settings::load(&settings_path(cli.config)?)?, &source)?,
            0,
        ),
        Some(Action::Merge(args)) => merge(args, cli.config)?,
        None => return Err(Error::usage("choose a subcommand or use --help")),
    };
    if cli.json {
        println!("{result}");
    } else {
        println!("{result:#}");
    }
    Ok(exit)
}

fn fail(error: Error, json_output: bool) -> ExitCode {
    eprintln!("fit-fusion: {}", error.message);
    if json_output {
        println!(
            "{}",
            json!({"schema_version":1,"status":"error","code":error.code,
            "message":error.message,"details":error.details,"artifacts":{},"next_action":null})
        );
    }
    ExitCode::from(error.exit)
}

fn main() -> ExitCode {
    let mut command = cli_help::command::<Cli>(
        "RustyFIT and wire reader",
        &format!("\nRules     {}", fit_fusion::RULES_VERSION),
    );
    let cli = match command
        .try_get_matches_from_mut(std::env::args_os())
        .and_then(|mut matches| Cli::from_arg_matches_mut(&mut matches))
    {
        Ok(cli) => cli,
        Err(mut error) => {
            if !error.use_stderr() {
                error.exit();
            }
            if error.kind() == ErrorKind::UnknownArgument
                && matches!(error.get(ContextKind::InvalidArg), Some(ContextValue::String(arg)) if arg == "--garmin")
            {
                error.insert(
                    ContextKind::SuggestedArg,
                    ContextValue::String("--auto-match".into()),
                );
                error.remove(ContextKind::Usage);
            }
            // A failed parse has no ArgMatches. Honor the agent's output format even
            // when clap encounters an invalid argument before reaching --json.
            let json_output = std::env::args_os()
                .skip(1)
                .take_while(|arg| arg != OsStr::new("--"))
                .any(|arg| arg == OsStr::new("--json"));
            if !json_output {
                error.exit();
            }
            return fail(Error::usage(error.to_string().trim_end()), true);
        }
    };
    let json = cli.json;
    match run(cli, &mut command) {
        Ok(exit) => ExitCode::from(exit),
        Err(error) => fail(error, json),
    }
}
