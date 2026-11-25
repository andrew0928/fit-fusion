//! Shared clap frontend for the source-distributed Python, C# and Node inspectors.
//! Launchers supply backend identity and an argv prefix. No shell is involved.
use clap::{Arg, ArgAction, Command};
use fit_inspector::sampling;
use serde_json::{Value, json};
use std::{
    env, io,
    process::{Command as Process, ExitCode, Stdio},
};

fn tree(language: &str, identity: &str) -> Command {
    let topics = if language == "node" {
        vec!["index", "inspect", "raw", "roundtrip", "sdk-check"]
    } else {
        vec!["index", "inspect", "raw", "roundtrip"]
    };
    let help = format!(
        "{identity} — {{about}}\n\n{{usage-heading}} {{usage}}\n\n{{all-args}}{{after-help}}"
    );
    let file = || Arg::new("file").value_name("FILE.fit").required(true);
    let mut inspect = Command::new("inspect")
        .about("Report field coverage and timing")
        .arg(file())
        .arg(
            Arg::new("stream")
                .long("stream")
                .value_name("NAME")
                .action(ArgAction::Append)
                .value_parser(sampling::selector)
                .help("Select MESSAGE.FIELD (repeatable)"),
        );
    if language == "python" {
        inspect = inspect.arg(
            Arg::new("decoder")
                .long("decoder")
                .value_parser(["auto", "wire", "fitdecode"])
                .default_value("auto"),
        );
    }
    let mut cmd = Command::new("fit-inspector")
        .about("Inspect one activity file offline")
        .help_template(help.clone())
        .arg_required_else_help(true)
        .args_conflicts_with_subcommands(true)
        .arg(
            Arg::new("version")
                .long("version")
                .short('V')
                .action(ArgAction::SetTrue)
                .help("Show runtime and decoder version"),
        )
        .arg(
            Arg::new("skill")
                .long("skill")
                .num_args(0..=1)
                .default_missing_value("index")
                .value_name("TOPIC")
                .value_parser(topics)
                .help("Read an offline agent guide"),
        )
        .subcommand(inspect)
        .subcommand(
            Command::new("raw")
                .about("Read FIT frames as JSON lines")
                .arg(file()),
        )
        .subcommand(
            Command::new("roundtrip")
                .about("Re-encode and verify a new FIT")
                .arg(file())
                .arg(
                    Arg::new("out")
                        .long("out")
                        .value_name("NEW.fit")
                        .required(true),
                ),
        );
    if language == "node" {
        cmd = cmd.subcommand(
            Command::new("sdk-check")
                .about("Verify Garmin SDK read-back")
                .arg(file()),
        );
    }
    cmd.mut_subcommands(|c| c.help_template(help.clone()))
}

fn run() -> io::Result<i32> {
    let args: Vec<_> = env::args().skip(1).collect();
    let divider = args
        .iter()
        .position(|s| s == "--")
        .filter(|i| *i >= 3)
        .ok_or_else(|| io::Error::other("expected backend, identity, launcher and --"))?;
    let language = &args[0];
    let m = tree(language, &args[1]).get_matches_from(
        std::iter::once("fit-inspector").chain(args[divider + 1..].iter().map(String::as_str)),
    );
    let mut streams = Vec::new();
    let mut request = json!({});
    let action = if m.get_flag("version") {
        "version"
    } else if let Some(topic) = m.get_one::<String>("skill") {
        request["topic"] = json!(topic);
        "skill"
    } else {
        let (cmd, opts) = m.subcommand().unwrap();
        request["file"] = json!(opts.get_one::<String>("file").unwrap());
        if cmd == "inspect" {
            if let Some(names) = opts.get_many::<String>("stream") {
                streams.extend(names.cloned());
            }
            if language == "python" {
                request["decoder"] = json!(opts.get_one::<String>("decoder"));
            }
        }
        if cmd == "roundtrip" {
            request["out"] = json!(opts.get_one::<String>("out"));
        }
        cmd
    };
    request["action"] = json!(action);
    let mut worker = Process::new(&args[2]);
    worker.args(&args[3..divider]).arg(request.to_string());
    if action != "inspect" {
        return Ok(worker.status()?.code().unwrap_or(1));
    }
    let output = worker.stderr(Stdio::inherit()).output()?;
    if !output.status.success() {
        return Ok(output.status.code().unwrap_or(1));
    }
    let mut report: Value = serde_json::from_slice(&output.stdout)?;
    sampling::annotate(&mut report, &streams);
    println!("{report}");
    Ok(0)
}
fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(e) => {
            eprintln!("fit-inspector: {e}");
            ExitCode::FAILURE
        }
    }
}
