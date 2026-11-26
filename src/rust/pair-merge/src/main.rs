#[path = "../../cli_help.rs"]
mod cli_help;
#[path = "../../effective_args.rs"]
mod effective_args;
mod merge;

use clap::{FromArgMatches, Parser, ValueEnum};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Clone, Copy, ValueEnum)]
enum Skill {
    Index,
    Author,
    Preflight,
    Merge,
}

#[derive(Parser)]
#[command(
    about = "Overwrite selected fields from one FIT onto another",
    arg_required_else_help = true,
    after_help = "Settings\n  Inputs come from CLI parameters or built-in defaults. Effective values are listed on stderr.\n\nExample\n  fit-merge --primary bike.fit --overlay band.fit --stream record.heart_rate --out ride.fit"
)]
struct Cli {
    /// Primary FIT providing the activity timeline
    #[arg(long, value_name = "FILE", required_unless_present = "skill")]
    primary: Option<PathBuf>,
    /// FIT providing the complete replacement streams
    #[arg(long, value_name = "FILE", required_unless_present = "skill")]
    overlay: Option<PathBuf>,
    /// Native FIT field to replace (repeat for each field)
    #[arg(
        long = "stream",
        value_name = "NAME",
        required_unless_present = "skill"
    )]
    streams: Vec<String>,
    /// New FIT file to create
    #[arg(long, value_name = "FILE", required_unless_present = "skill")]
    out: Option<PathBuf>,
    /// Return the output path as JSON
    #[arg(long)]
    json: bool,
    /// Read an offline agent guide (default: index)
    #[arg(long, value_name="TOPIC", num_args=0..=1, default_missing_value="index",
          conflicts_with_all=["primary","overlay","streams","out","json"])]
    skill: Option<Skill>,
}

fn read(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", path.display())))
}

fn export(primary: &Path, overlay: &Path, streams: &[String], out: &Path) -> io::Result<()> {
    let bytes = merge::merge(&read(primary)?, &read(overlay)?, streams)?;
    let parent = out
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(&bytes)?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(out).map_err(|e| e.error)?;
    Ok(())
}

fn main() -> ExitCode {
    let tree = cli_help::command::<Cli>("FIT wire reader/writer", "");
    let matches = tree.clone().get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    if let Some(topic) = cli.skill {
        print!(
            "{}",
            match topic {
                Skill::Index => include_str!("../guides/index.md"),
                Skill::Author => cli_help::AUTHOR_GUIDE,
                Skill::Preflight => include_str!("../guides/preflight.md"),
                Skill::Merge => include_str!("../guides/merge.md"),
            }
        );
        return ExitCode::SUCCESS;
    }
    effective_args::report(&tree, &matches, &Default::default());
    let out = cli.out.expect("clap requires --out");
    match export(
        &cli.primary.expect("clap requires --primary"),
        &cli.overlay.expect("clap requires --overlay"),
        &cli.streams,
        &out,
    ) {
        Ok(()) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"status":"exported","artifacts":{"fit":out}})
                );
            } else {
                println!("{}", out.display());
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            if cli.json {
                println!(
                    "{}",
                    serde_json::json!({"status":"error","message":e.to_string()})
                );
            }
            eprintln!("fit-merge: {e}");
            ExitCode::from(3)
        }
    }
}
