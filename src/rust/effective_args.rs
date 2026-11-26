//! Show the effective inputs on stderr before starting a data command.
use clap::{Arg, ArgAction, ArgMatches, Command, parser::ValueSource};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    env,
    io::{self, IsTerminal, Write},
    path::PathBuf,
};
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub struct Origins {
    pub file: Option<Value>,
    /// Argument ID to fully qualified YAML key, for defaults supplied by the loader.
    pub keys: BTreeMap<String, String>,
}

fn printable(s: &str) -> String {
    let mut clean = String::new();
    for c in s.chars() {
        if c.is_control() {
            clean.extend(c.escape_default());
        } else {
            clean.push(c);
        }
    }
    clean
}

fn parameter_value(arg: &Arg, args: &ArgMatches) -> Option<Value> {
    let id = arg.get_id().as_str();
    if let Ok(Some(path)) = args.try_get_one::<PathBuf>(id) {
        let path = std::path::absolute(path).unwrap_or_else(|_| path.clone());
        return Some(json!(path.to_string_lossy()));
    }
    if matches!(arg.get_action(), ArgAction::SetTrue) {
        return Some(json!(args.get_flag(id)));
    }
    let mut raw = args.get_raw(id)?;
    if matches!(arg.get_action(), ArgAction::Append) {
        return Some(json!(raw.map(|v| v.to_string_lossy()).collect::<Vec<_>>()));
    }
    if let Ok(Some(value)) = args.try_get_one::<String>(id) {
        return Some(json!(value));
    }
    raw.next().map(|v| json!(v.to_string_lossy()))
}

fn collect_args(
    cmd: &Command,
    args: &ArgMatches,
    origins: &Origins,
    offline: bool,
    parameters: &mut BTreeMap<String, Value>,
) {
    for arg in cmd.get_arguments() {
        let id = arg.get_id().as_str();
        if matches!(id, "help" | "version" | "skill" | "json" | "text")
            || (offline && id == "env_file")
        {
            continue;
        }
        let Some(origin) = args.value_source(id) else {
            continue;
        };
        let Some(value) = parameter_value(arg, args) else {
            continue;
        };
        let key = arg.get_long().unwrap_or(id);
        let (source, detail) = if origin == ValueSource::CommandLine {
            (
                "cli",
                arg.get_long()
                    .map(|flag| format!("--{flag}"))
                    .unwrap_or_else(|| id.into()),
            )
        } else if let Some(config_key) = origins.keys.get(id) {
            ("config", config_key.clone())
        } else {
            ("cli", "default".into())
        };
        parameters.insert(
            key.into(),
            json!({"value":value,"source":source,"detail":detail}),
        );
    }
}

pub fn report(tree: &Command, matches: &ArgMatches, origins: &Origins) {
    let mut name = tree.get_name().to_owned();
    let mut parameters = BTreeMap::new();
    let offline = matches
        .subcommand()
        .is_some_and(|(_, sub)| sub.try_get_one::<bool>("offline").ok().flatten() == Some(&true));
    collect_args(tree, matches, origins, offline, &mut parameters);
    if let Some((sub, args)) = matches.subcommand() {
        name.push(' ');
        name.push_str(sub);
        collect_args(
            tree.find_subcommand(sub).expect("parsed command"),
            args,
            origins,
            offline,
            &mut parameters,
        );
    }
    let flag = |key| matches.try_get_one::<bool>(key).ok().flatten() == Some(&true);
    let compact = flag("json");
    let (format, detail) = if compact {
        ("json", "--json")
    } else if flag("text") {
        ("text", "--text")
    } else if tree.get_name().starts_with("fit-query-") {
        ("indented-json", "default")
    } else if tree.get_name() == "fit-merge" {
        ("path", "default")
    } else if matches.subcommand_name() == Some("raw") {
        ("jsonl", "default")
    } else {
        ("json", "default")
    };
    parameters.insert(
        "output".to_owned(),
        json!({"value":format,"source":"cli","detail":detail}),
    );

    let stderr = io::stderr();
    let color = stderr.is_terminal()
        && env::var_os("NO_COLOR").is_none_or(|s| s.is_empty())
        && env::var_os("TERM").is_none_or(|s| s != "dumb");
    let mut stream = stderr.lock();
    if compact {
        let _ = writeln!(
            stream,
            "{}",
            json!({"command":name,"config":origins.file,"parameters":parameters})
        );
        return;
    }
    let rows: Vec<_> = parameters
        .iter()
        .map(|(key, p)| {
            let value = p["value"]
                .as_str()
                .map(printable)
                .unwrap_or_else(|| p["value"].to_string());
            let source = if p["detail"] == "default" {
                "cli (default)"
            } else {
                p["source"].as_str().unwrap()
            };
            (key, value, source)
        })
        .collect();
    let keys = rows.iter().map(|(k, _, _)| k.len()).max().unwrap_or(0);
    let values = rows.iter().map(|(_, v, _)| v.width()).max().unwrap_or(0);
    for (key, value, source) in rows {
        let pad = values - value.width();
        if color {
            let _ = writeln!(
                stream,
                "- \x1b[31m{key:keys$}\x1b[0m  \x1b[37m{value}{space:pad$}\x1b[0m  \x1b[90m{source}\x1b[0m",
                space = ""
            );
        } else {
            let _ = writeln!(
                stream,
                "- {key:keys$}  {value}{space:pad$}  {source}",
                space = ""
            );
        }
    }
}
