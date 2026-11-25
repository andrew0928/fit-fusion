mod catalog;
#[path = "../../cli_help.rs"]
mod cli_help;
mod export;
#[path = "../../query_output.rs"]
mod query_output;

use catalog::{FileError, Listing, Query};
use clap::builder::TypedValueParser;
use clap::{FromArgMatches, Parser, Subcommand, ValueEnum, error::ErrorKind};
#[path = "../../query_filters.rs"]
mod query_filters;
use jiff::tz::TimeZone;
use query_filters::{Filters, parse_zone};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    about = "Find, read and export original cycling FIT files",
    arg_required_else_help = true,
    after_help = "Examples\n  fit-query-files --dir rides list --date 2026-11-21 --json\n  fit-query-files --dir rides export --date 2026-11-21 --out selected\n  fit-query-files --skill"
)]
struct Cli {
    /// Directory of FIT files (searched recursively)
    #[arg(long, global = true, value_name = "DIR")]
    dir: Option<PathBuf>,
    /// IANA zone for the ride's departure date
    #[arg(long, global=true, default_value="Asia/Taipei", value_parser=parse_zone, value_name="ZONE")]
    timezone: TimeZone,
    #[command(flatten)]
    output: query_output::Output,
    /// Read an offline agent guide
    #[arg(long, num_args=0..=1, default_missing_value="index", value_name="TOPIC", conflicts_with_all=["dir","json","text"])]
    skill: Option<Skill>,
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Skill {
    Index,
    List,
    Read,
    Export,
}

#[derive(Subcommand)]
enum Action {
    /// List matching rides in departure order
    #[command(
        after_help = "Example\n  fit-query-files --dir rides list --from 2026-11-01 --until 2026-12-01 --json"
    )]
    List(Filters),
    /// Read activity metadata and record-stream time ranges
    #[command(after_help = r#"Example
  id=$(fit-query-files --dir rides list --json | jq -er '.activities[0].id')
  fit-query-files --dir rides read --id "$id" --json"#)]
    Read {
        /// Activity ID from list (SHA-256)
        #[arg(long, value_parser=parse_id)]
        id: String,
    },
    /// Copy each matching original FIT into a directory
    #[command(
        after_help = "Example\n  fit-query-files --dir rides export --date 2026-11-21 --out selected --json"
    )]
    Export {
        /// Select one activity by its SHA-256
        #[arg(long, value_parser=parse_id, conflicts_with_all=["date","from","until","device"])]
        id: Option<String>,
        #[command(flatten)]
        filters: Filters,
        /// Destination directory (created when exporting)
        #[arg(long, value_name = "DIR", value_parser = clap::builder::StringValueParser::new().map(PathBuf::from))]
        out: PathBuf,
    },
}

fn parse_id(text: &str) -> Result<String, String> {
    if text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(text.to_ascii_lowercase())
    } else {
        Err("expected a 64-digit SHA-256 activity ID from list".into())
    }
}
impl Filters {
    fn query(&self, zone: TimeZone, id: Option<String>) -> Result<Query, String> {
        let days = self.days()?;
        Ok(Query {
            days,
            timezone: zone,
            device: self.device,
            id,
        })
    }
}

fn usage(message: &str) -> ! {
    cli_help::command::<Cli>("FIT wire reader", "")
        .error(ErrorKind::InvalidValue, message)
        .exit()
}
fn run(dir: &Path, action: &Action, zone: TimeZone) -> Value {
    let defaults = Filters::default();
    let (filters, id, output) = match action {
        Action::List(filters) => (filters, None, None),
        Action::Read { id } => (&defaults, Some(id), None),
        Action::Export { filters, id, out } => (filters, id.as_ref(), Some(out.as_path())),
    };
    let query = filters
        .query(zone, id.cloned())
        .unwrap_or_else(|msg| usage(&msg));
    let Listing {
        activities,
        mut errors,
    } = match catalog::list(dir, &query, output) {
        Ok(listing) => listing,
        Err(e) => {
            return json!({"status":"error","errors":[FileError{path:dir.into(),message:e.to_string()}]});
        }
    };
    if activities.is_empty()
        && let Some(id) = id
        && errors.is_empty()
    {
        errors.push(FileError {
            path: dir.into(),
            message: format!("activity not found: {id}"),
        });
    }
    let mut success_count = activities.len();
    let mut result = match action {
        Action::List(_) => json!({"activities":activities}),
        Action::Read { .. } => match activities.first() {
            Some(activity) => match activity.streams() {
                Ok(streams) => json!({"activity":activity,"streams":streams}),
                Err(e) => {
                    success_count = 0;
                    errors.push(FileError {
                        path: activity.path.clone(),
                        message: e.to_string(),
                    });
                    json!({})
                }
            },
            None => json!({}),
        },
        Action::Export { out, .. } => {
            let items: Vec<_> = activities.iter().map(|a| export::export(a, out)).collect();
            success_count = items.iter().filter(|item| item.message.is_none()).count();
            for item in &items {
                if let Some(message) = &item.message {
                    errors.push(FileError {
                        path: item.path.clone(),
                        message: message.clone(),
                    });
                }
            }
            json!({"exports":items})
        }
    };
    result["status"] = json!(if errors.is_empty() {
        "ok"
    } else if success_count > 0 {
        "partial"
    } else {
        "error"
    });
    result["errors"] = json!(errors);
    result
}
fn main() -> ExitCode {
    let mut matches = cli_help::command::<Cli>("FIT wire reader", "").get_matches();
    let cli = Cli::from_arg_matches_mut(&mut matches).unwrap_or_else(|e| e.exit());
    cli.output.validate().unwrap_or_else(|e| e.exit());
    if let Some(topic) = cli.skill {
        if cli.action.is_some() {
            usage("--skill must be used on its own");
        }
        print!(
            "{}",
            match topic {
                Skill::Index => include_str!("../guides/index.md"),
                Skill::List => include_str!("../guides/list.md"),
                Skill::Read => include_str!("../guides/read.md"),
                Skill::Export => include_str!("../guides/export.md"),
            }
        );
        return ExitCode::SUCCESS;
    }
    let action = cli
        .action
        .unwrap_or_else(|| usage("expected list, read or export"));
    let dir = cli.dir.unwrap_or_else(|| usage("provide --dir DIR"));
    let result = run(&dir, &action, cli.timezone.clone());
    cli.output.print(&result, &cli.timezone);
    if result["status"] == "ok" {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(3)
    }
}

#[cfg(test)]
mod tests;
