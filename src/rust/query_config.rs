//! Personal settings shared by the three independent query binaries.
use crate::effective_args::Origins;
use clap::{ArgMatches, Command, error::ErrorKind};
use serde_json::json;
use serde_yaml_ng::Value;
use std::{
    env, fs,
    io::{self, Write},
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
};

const TEMPLATE: &str = include_str!("../../docs/config/config.example.yaml");
pub const GUIDE: &str = include_str!("query-config-guide.md");

fn config_path() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".fit-fusion/config.yaml"))
}

fn init(path: &Path) -> io::Result<&'static str> {
    let parent = path.parent().expect("config directory");
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(parent)?;
    let existing = || -> io::Result<&'static str> {
        if fs::symlink_metadata(path)?.is_file() {
            Ok("already_exists")
        } else {
            Err(io::Error::other("destination is not a regular file"))
        }
    };
    match existing() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        result => return result,
    }
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(TEMPLATE.as_bytes())?;
    file.as_file().sync_all()?;
    match file.persist_noclobber(path) {
        Ok(_) => Ok("created"),
        Err(e) if e.error.kind() == io::ErrorKind::AlreadyExists => existing(),
        Err(e) => Err(e.error),
    }
}

fn read(path: &Path) -> Result<(Value, &'static str), String> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok((Value::Null, "missing")),
        Err(e) => return Err(e.to_string()),
    };
    if text
        .lines()
        .all(|s| s.trim().is_empty() || s.trim_start().starts_with('#'))
    {
        return Ok((Value::Null, "empty"));
    }
    let root: Value = serde_yaml_ng::from_str(&text).map_err(|e| match e.location() {
        Some(at) => format!("invalid YAML at line {}, column {}", at.line(), at.column()),
        None => "invalid YAML".to_string(),
    })?;
    if !root.is_mapping() || root["version"].as_u64() != Some(1) {
        return Err("expected a mapping with integer version: 1".into());
    }
    Ok((root, "loaded"))
}

fn section<'a>(root: &'a Value, key: &str) -> Result<&'a Value, String> {
    match root.get(key) {
        Some(v) if !v.is_mapping() => Err(format!("{key} must be a mapping")),
        Some(v) => Ok(v),
        None => Ok(&Value::Null),
    }
}

fn configure(
    mut tree: Command,
    tool: &str,
    args: &ArgMatches,
    path: &Path,
    origins: &mut Origins,
) -> Result<Command, String> {
    let (root, status) = read(path)?;
    origins.file = Some(json!({"path":path,"status":status}));
    let common = section(&root, "common")?;
    let own = section(section(&root, "tools")?, tool)?;
    let mut settings = vec![("timezone", common, "common".to_owned())];
    let fields: &[&str] = match tool {
        "fit-query-files" => &["dir"],
        "fit-query-apple" => &["archive"],
        "fit-query-garmin" => &["env_file", "cache_dir"],
        _ => unreachable!("query tool name"),
    };
    for key in fields {
        settings.push((key, own, format!("tools.{tool}")));
    }
    for (key, map, section) in settings {
        let qualified = format!("{section}.{key}");
        let Some(value) = map.get(key) else { continue };
        let text = value
            .as_str()
            .ok_or_else(|| format!("{qualified} must be a string"))?;
        if args.value_source(key) == Some(clap::parser::ValueSource::CommandLine) {
            continue;
        }
        let value = if key == "timezone" {
            jiff::tz::TimeZone::get(text)
                .map_err(|_| format!("{qualified} must be a valid IANA time zone"))?;
            text.to_owned()
        } else {
            let base = path.parent().expect("config directory");
            let resolved = match text.strip_prefix("~/") {
                Some(suffix) => base.parent().expect("home directory").join(suffix),
                None => base.join(text),
            };
            resolved.to_string_lossy().into_owned()
        };
        origins.keys.insert(key.to_owned(), qualified);
        tree = tree.mut_arg(key, |arg| arg.default_value(value));
    }
    Ok(tree)
}

pub fn parse(tree: Command) -> (Command, ArgMatches, Origins) {
    let mut tree = tree.subcommand(crate::cli_help::header(
        Command::new("config")
            .about("Manage personal YAML settings")
            .subcommand_required(true)
            .subcommand(Command::new("init").about("Create a commented example if absent")),
    ));
    // Parse syntax, help and skill before touching the user's configuration.
    let args = tree.clone().get_matches();
    let mut origins = Origins::default();
    if args.contains_id("skill") {
        if args.subcommand().is_some() {
            tree.error(
                ErrorKind::ArgumentConflict,
                "--skill must be used on its own",
            )
            .exit();
        }
        return (tree, args, origins);
    }
    let path = config_path();
    if args.subcommand_name() == Some("config") {
        let path = path.unwrap_or_else(|| {
            tree.error(ErrorKind::InvalidValue, "config init requires HOME")
                .exit()
        });
        if args.get_flag("text") && args.get_flag("json") {
            tree.error(
                ErrorKind::ArgumentConflict,
                "--json and --text cannot be used together",
            )
            .exit();
        }
        match init(&path) {
            Ok(status) => {
                if args.get_flag("text") {
                    println!("{status}  {}", path.display());
                } else {
                    println!("{}", json!({"status":status,"path":path}));
                }
                std::process::exit(0);
            }
            Err(e) => tree
                .error(ErrorKind::Io, format!("{}: {e}", path.display()))
                .exit(),
        }
    }
    if args.subcommand().is_none() {
        return (tree, args, origins);
    }
    let name = tree.get_name().to_owned();
    if let Some(ref path) = path {
        if name == "fit-query-garmin" {
            let cache = path
                .parent()
                .and_then(Path::parent)
                .unwrap()
                .join("Library/Caches/fit-query-garmin");
            tree = tree.mut_arg("cache_dir", |a| a.default_value(cache.into_os_string()));
        }
        tree = configure(tree.clone(), &name, &args, path, &mut origins).unwrap_or_else(|e| {
            tree.error(ErrorKind::InvalidValue, format!("{}: {e}", path.display()))
                .exit()
        });
    }
    // clap applies command-line values above the configured defaults and runs the
    // same typed parsers (including time zones) for both sources.
    let args = tree.clone().get_matches();
    if let Some((_, sub)) = args.subcommand() {
        let date = |id| sub.try_get_one::<jiff::civil::Date>(id).ok().flatten();
        if let (Some(start), Some(end)) = (date("from"), date("until"))
            && end <= start
        {
            tree.error(ErrorKind::InvalidValue, "--until must be after --from")
                .exit();
        }
    }
    (tree, args, origins)
}
