use crate::guidance::Skill;
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    about = "Merge cycling recordings into one FIT",
    arg_required_else_help = true,
    after_help = "Examples\n  fit-fusion config init\n  fit-fusion merge --date 2026-11-21 --json\n  fit-fusion --skill index"
)]
pub struct Cli {
    /// Use this settings file (default: ~/.config/fit-fusion/config.json)
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,
    /// Emit compact JSON for operations and structured errors on failure
    #[arg(long, global = true)]
    pub json: bool,
    /// Open an offline agent guide (default: index)
    #[arg(long, num_args = 0..=1, default_missing_value = "index", value_name = "TOPIC", hide_possible_values = true)]
    pub skill: Option<Skill>,
    #[command(subcommand)]
    pub command: Option<Action>,
}

#[derive(Subcommand)]
pub enum Action {
    /// Manage personal settings
    #[command(after_help = "Examples\n  fit-fusion config init\n  fit-fusion config check --json")]
    Config {
        #[command(subcommand)]
        command: Option<Config>,
    },
    /// Discover cycling activities by source
    #[command(
        after_help = "Examples\n  fit-fusion config check\n  fit-fusion activities list --source bike_meter --json"
    )]
    Activities {
        #[command(subcommand)]
        command: Option<Activities>,
    },
    /// Combine recordings of the same ride
    #[command(
        after_help = "Examples\n  fit-fusion merge --date 2026-11-21 --json\n  fit-fusion merge bike.fit band.fit --out merged-ride\n  fit-fusion merge bike.fit --auto-match --out matched-ride\n  fit-fusion merge bike.fit --garmin-id 12345 --out chosen-ride"
    )]
    Merge(Merge),
}

#[derive(Subcommand)]
pub enum Config {
    /// Create a private settings template, keeping any existing file
    Init,
    /// Validate settings offline and show source IDs and explicit stream overrides
    Check,
}

#[derive(Subcommand)]
pub enum Activities {
    /// List cycling activities from a local FIT directory or Garmin Connect
    List {
        /// Source ID from config check
        #[arg(long, value_name = "ID")]
        source: String,
    },
}

#[derive(Args)]
pub struct Merge {
    /// Find rides by departure day in the configured time zone
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = fit_fusion::config::parse_day,
          conflicts_with_all = ["first", "second", "out", "auto_match", "garmin_id"])]
    pub date: Option<jiff::civil::Date>,
    /// A local recording (the primary FIT in single-file mode)
    #[arg(value_name = "FIRST.fit", required_unless_present = "date")]
    pub first: Option<PathBuf>,
    /// The other recording for an explicit pair, in either device order
    #[arg(value_name = "SECOND.fit", required_unless_present_any = ["date", "auto_match", "garmin_id"],
          conflicts_with_all = ["auto_match", "garmin_id"])]
    pub second: Option<PathBuf>,
    /// Find the same ride in sources referenced by merge.streams
    #[arg(long, conflicts_with = "garmin_id")]
    pub auto_match: bool,
    /// Select a known companion from the configured Garmin source
    #[arg(long, value_name = "ID")]
    pub garmin_id: Option<String>,
    /// Destination for manual modes (date mode uses output_dir)
    #[arg(long, value_name = "DIR", required_unless_present = "date")]
    pub out: Option<PathBuf>,
    /// Also export source frames and selected samples for analysis
    #[arg(long)]
    pub parquet: bool,
}
