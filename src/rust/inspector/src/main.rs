#[path = "../../cli_help.rs"]
mod cli_help;
#[path = "../../effective_args.rs"]
mod effective_args;
mod guidance;
mod roundtrip;

use clap::{FromArgMatches, Parser, Subcommand, ValueEnum};
use fit_inspector::{inventory, wire};
use std::{
    fs,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    about = "Inspect one activity file offline",
    arg_required_else_help = true,
    args_conflicts_with_subcommands = true,
    after_help = "Settings\n  Inputs come from CLI parameters or built-in defaults. Effective values are listed on stderr.\n\nExample\n  fit-inspector inspect ride.fit"
)]
struct Cli {
    /// Open an offline agent guide (default: index)
    #[arg(long, num_args = 0..=1, default_missing_value = "index", value_name = "TOPIC", hide_possible_values = true)]
    skill: Option<guidance::Skill>,
    #[command(subcommand)]
    action: Option<Action>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Decoder {
    Rustyfit,
    Wire,
}

#[derive(Subcommand)]
enum Action {
    /// Summarize messages, fields, sample coverage and timing as JSON
    Inspect {
        /// Input recording
        #[arg(value_name = "FILE.fit")]
        file: PathBuf,
        /// Report coverage for MESSAGE.FIELD (repeatable; default: all actual fields)
        #[arg(long="stream",value_name="NAME",value_parser=fit_inspector::sampling::selector)]
        streams: Vec<String>,
        /// Inventory decoder
        #[arg(long, value_enum, default_value = "rustyfit")]
        decoder: Decoder,
    },
    /// Validate and print one JSON line per FIT frame
    Raw {
        /// Input recording
        #[arg(value_name = "FILE.fit")]
        file: PathBuf,
    },
    /// Write a new FIT and verify its decoded values
    Roundtrip {
        /// Input recording
        #[arg(value_name = "FILE.fit")]
        file: PathBuf,
        /// New FIT path in an existing directory
        #[arg(long, value_name = "NEW.fit")]
        out: PathBuf,
    },
}

fn run(action: &Action) -> io::Result<()> {
    let file = match action {
        Action::Inspect { file, .. } | Action::Raw { file } | Action::Roundtrip { file, .. } => {
            file
        }
    };
    let execute = || -> io::Result<()> {
        let data = fs::read(file)?;
        let mut stdout = io::BufWriter::new(io::stdout().lock());
        match action {
            Action::Raw { .. } => wire::walk(&data, |frame| {
                serde_json::to_writer(&mut stdout, &frame.raw())?;
                writeln!(stdout)
            })?,
            Action::Inspect {
                decoder, streams, ..
            } => {
                let mut report = inventory::inspect(
                    &data,
                    &file.to_string_lossy(),
                    matches!(decoder, Decoder::Rustyfit),
                )?;
                fit_inspector::sampling::annotate(&mut report, streams);
                serde_json::to_writer(&mut stdout, &report)?;
                writeln!(stdout)?;
            }
            Action::Roundtrip { out, .. } => {
                serde_json::to_writer(&mut stdout, &roundtrip::export(&data, out)?)?;
                writeln!(stdout)?;
            }
        }
        stdout.flush()
    };
    execute().map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", file.display())))
}

fn main() -> ExitCode {
    let tree = cli_help::command::<Cli>("RustyFIT and wire reader", "");
    let matches = tree.clone().get_matches();
    let cli = Cli::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());
    if let Some(topic) = cli.skill {
        print!("{}", topic.page());
        return ExitCode::SUCCESS;
    }
    effective_args::report(&tree, &matches, &Default::default());
    match run(&cli.action.expect("clap requires a command or --skill")) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fit-inspector: {e}");
            ExitCode::FAILURE
        }
    }
}
