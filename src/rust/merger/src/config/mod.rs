//! Offline configuration: validate first, connect to sources only on demand.
mod unique_json;
use crate::{
    Error, Result,
    data_source::{DataSource, DeferredSource, Device, FitDirectory, SourceKind},
    policy::MergePolicy,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
    sync::LazyLock,
};

const EXAMPLE: &str = include_str!("../../../../../docs/config/config.example.json");
static SCHEMA: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    let spec = serde_json::from_str(include_str!(
        "../../../../../docs/config/config-v1.schema.json"
    ))
    .expect("embedded schema JSON");
    jsonschema::draft7::new(&spec).expect("embedded config v1 schema")
});

pub fn parse_day(day: &str) -> Result<jiff::civil::Date> {
    use jiff::civil::Date;
    let date: Date = day
        .parse()
        .map_err(|_| Error::usage("expected YYYY-MM-DD"))?;
    if date.to_string() != day {
        return Err(Error::usage("expected YYYY-MM-DD"));
    }
    Ok(date)
}

pub fn invalid(at: &str, why: &str) -> Error {
    let mut e = Error::new(
        "invalid_config",
        format!("{}: {why}", if at.is_empty() { "/" } else { at }),
        2,
    );
    e.details = json!({"path":at});
    e
}
fn resolve(base: &Path, s: &str) -> Result<PathBuf> {
    if s == "~" || s.starts_with("~/") {
        let home = std::env::home_dir()
            .ok_or_else(|| invalid("", "home directory unavailable for ~ path"))?;
        Ok(home.join(s.strip_prefix("~/").unwrap_or("")))
    } else {
        Ok(base.join(s))
    }
}

fn normalized(path: PathBuf) -> Result<PathBuf> {
    let mut result = PathBuf::new();
    for c in std::path::absolute(path)?.components() {
        match c {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            _ => {
                result.push(c.as_os_str());
                if result.is_symlink() {
                    result = std::fs::canonicalize(&result)
                        .map_err(|_| invalid("", "cannot resolve configured symbolic link"))?;
                }
            }
        }
    }
    Ok(result)
}
fn check_paths(v: &Value, base: &Path) -> Result<()> {
    let paths = v["sources"].as_object().unwrap();
    let output = normalized(resolve(base, v["output_dir"].as_str().unwrap())?)?;
    let mut writes = vec![("/output_dir".to_owned(), output)];
    for (id, s) in paths {
        if s["kind"] == "garmin_connect" {
            writes.push((
                format!("/sources/{id}/cache_dir"),
                normalized(resolve(base, s["cache_dir"].as_str().unwrap())?)?,
            ));
        }
    }
    for s in paths.values().filter(|s| s["kind"] == "fit_directory") {
        let input = normalized(resolve(base, s["directory"].as_str().unwrap())?)?;
        for (at, dir) in &writes {
            if dir.starts_with(&input) || input.starts_with(dir) {
                return Err(invalid(at, "output or cache overlaps an input directory"));
            }
        }
    }
    Ok(())
}
struct SourceContext<'a> {
    pub base: &'a Path,
    pub config_path: Option<&'a Path>,
}
type SourceFactory = fn(&str, &Value, &Path, &Registry) -> Result<Box<dyn DataSource>>;
type ContextFactory =
    dyn Fn(&str, &Value, &SourceContext<'_>, &Registry) -> Result<Box<dyn DataSource>>;
type TargetFactory = fn(&str, &Value) -> Result<()>;
pub struct Registry {
    devices: BTreeMap<String, Device>,
    sources: BTreeMap<String, Box<ContextFactory>>,
    targets: BTreeMap<String, TargetFactory>,
}
impl Default for Registry {
    fn default() -> Self {
        let mut r = Self {
            devices: BTreeMap::new(),
            sources: BTreeMap::new(),
            targets: BTreeMap::new(),
        };
        r.register_device("bryton_rider_460", Device::RIDER_460);
        r.register_device("garmin_cirqa", Device::CIRQA);
        r.register_source("fit_directory", |id, v, base, r| {
            let device = v.get("device").map(|_| r.device(id, v)).transpose()?;
            Ok(Box::new(FitDirectory::new(
                id,
                resolve(base, v["directory"].as_str().unwrap())?,
                device,
            )))
        });
        r.sources.insert(
            "garmin_connect".into(),
            Box::new(|id, v, context, r| {
                if r.device(id, v)?.manufacturer != Device::CIRQA.manufacturer {
                    return Err(invalid(
                        &format!("/sources/{id}/device"),
                        "incompatible Garmin device",
                    ));
                }
                Ok(Box::new(crate::data_source::GarminConnect::new(
                    id,
                    r.device(id, v)?,
                    resolve(context.base, v["cache_dir"].as_str().unwrap())?,
                    context.config_path,
                    &v["auth"],
                )))
            }),
        );
        r.register_source("apple_health_export", |id, _, _, _| {
            Ok(Box::new(DeferredSource {
                id: id.into(),
                kind: SourceKind::AppleHealthExport,
            }))
        });
        r.register_target("strava", |_, _| Ok(()));
        r
    }
}
impl Registry {
    pub fn register_device(&mut self, name: &str, device: Device) {
        self.devices.insert(name.into(), device);
    }
    pub fn register_source(&mut self, kind: &str, create: SourceFactory) {
        self.sources.insert(
            kind.into(),
            Box::new(move |id, v, ctx, r| create(id, v, ctx.base, r)),
        );
    }
    pub fn register_target(&mut self, kind: &str, validate: TargetFactory) {
        self.targets.insert(kind.into(), validate);
    }
    fn device(&self, id: &str, v: &Value) -> Result<Device> {
        if let Some(identity) = v["device"].as_object() {
            let get = |key: &str| {
                identity
                    .get(key)?
                    .as_u64()
                    .and_then(|n| u16::try_from(n).ok())
            };
            return get("manufacturer")
                .zip(get("product"))
                .map(|(manufacturer, product)| Device {
                    manufacturer,
                    product,
                })
                .ok_or_else(|| {
                    invalid(
                        &format!("/sources/{id}/device"),
                        "invalid FIT device identity",
                    )
                });
        }
        self.devices
            .get(v["device"].as_str().unwrap_or_default())
            .copied()
            .ok_or_else(|| invalid(&format!("/sources/{id}/device"), "unregistered device"))
    }
}
pub struct Target {
    pub id: String,
    pub kind: String,
}
pub struct Settings {
    sources: BTreeMap<String, Box<dyn DataSource>>,
    targets: Vec<Target>,
    devices: BTreeMap<String, Device>,
    record_fields: BTreeMap<String, u8>,
    selection: Value,
    pub timezone: String,
    pub output_dir: PathBuf,
}
impl Settings {
    /// Install the credential-free v1 example without replacing an existing config.
    pub fn init(path: &Path) -> Result<bool> {
        use std::{io::Write, os::unix::fs::DirBuilderExt};
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(parent)?;
        let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
        tmp.write_all(EXAMPLE.as_bytes())?;
        tmp.as_file().sync_all()?;
        match tmp.persist_noclobber(path) {
            Ok(_) => Ok(true),
            Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
            Err(e) => Err(e.error.into()),
        }
    }
    pub fn load(path: &Path) -> Result<Self> {
        Self::load_with(path, &Registry::default())
    }
    pub fn load_with(path: &Path, registry: &Registry) -> Result<Self> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| invalid("", &format!("{}: {e}", path.display())))?;
        Self::parse_at(
            &text,
            path.parent().unwrap_or(Path::new(".")),
            Some(path),
            registry,
        )
    }
    pub(crate) fn embedded() -> Result<Self> {
        Self::parse(EXAMPLE, Path::new("."), &Registry::default())
    }
    fn parse(text: &str, base: &Path, r: &Registry) -> Result<Self> {
        Self::parse_at(text, base, None, r)
    }
    fn parse_at(text: &str, base: &Path, config_path: Option<&Path>, r: &Registry) -> Result<Self> {
        let v = unique_json::parse(text).map_err(|e| {
            invalid(
                "",
                &format!("invalid JSON at line {}, column {}", e.line(), e.column()),
            )
        })?;
        if let Err(e) = SCHEMA.validate(&v) {
            return Err(invalid(
                &e.instance_path.to_string(),
                "does not match config v1 schema",
            ));
        }
        let timezone = v["timezone"].as_str().unwrap();
        if jiff::tz::TimeZone::get(timezone).is_err() {
            return Err(invalid("/timezone", "unknown IANA time zone"));
        }
        check_paths(&v, base)?;
        let mut sources = BTreeMap::new();
        let definitions = v["sources"].as_object().unwrap();
        for (id, v) in definitions {
            let factory = r.sources.get(v["kind"].as_str().unwrap()).ok_or_else(|| {
                invalid(&format!("/sources/{id}/kind"), "unregistered source kind")
            })?;
            sources.insert(
                id.clone(),
                factory(id, v, &SourceContext { base, config_path }, r)?,
            );
        }
        let mut targets = Vec::new();
        if let Some(items) = v["destinations"].as_object() {
            for (id, v) in items {
                let kind = v["kind"].as_str().unwrap_or("strava");
                let factory = r.targets.get(kind).ok_or_else(|| {
                    invalid(
                        &format!("/destinations/{id}/kind"),
                        "unregistered target kind",
                    )
                })?;
                factory(id, v)?;
                targets.push(Target {
                    id: id.clone(),
                    kind: kind.into(),
                });
            }
        }
        let mut merge = v["merge"].clone();
        merge
            .as_object_mut()
            .expect("validated merge object")
            .entry("streams")
            .or_insert_with(|| json!({}));
        let primary = merge["primary_source"].as_str().unwrap();
        if !sources.contains_key(primary) || definitions[primary]["kind"] == "apple_health_export" {
            return Err(invalid(
                "/merge/primary_source",
                "expected a configured cycling activity source",
            ));
        }
        let streams = merge["streams"].as_object().unwrap();
        let mut record_fields = BTreeMap::new();
        if let Some(aliases) = merge["field_ids"].as_object() {
            for (name, value) in aliases {
                let id = value.as_u64().unwrap() as u8;
                if crate::native::NAMES.contains_key(&(20, id))
                    || crate::native::record_id(name).is_some()
                    || record_fields.values().any(|&v| v == id)
                {
                    return Err(invalid(
                        &format!("/merge/field_ids/{name}"),
                        "use the canonical name for a known FIT field; private IDs must be unique",
                    ));
                }
                record_fields.insert(name.clone(), id);
            }
        }
        for (field, ids) in streams {
            if field.starts_with("record.") {
                let id = crate::native::record_id(field).or_else(|| record_fields.get(field).copied())
                    .filter(|id| *id < 253).ok_or_else(|| invalid(&format!("/merge/streams/{field}"), "unknown or structural record field; declare a new native ID in merge.field_ids"))?;
                record_fields.insert(field.clone(), id);
            }
            for (i, id) in ids.as_array().unwrap().iter().enumerate() {
                let source_id = id.as_str().unwrap();
                let at = format!("/merge/streams/{field}/{i}");
                let spec = definitions
                    .get(source_id)
                    .ok_or_else(|| invalid(&at, "undeclared source ID"))?;
                let allowed = spec["kind"] != "apple_health_export"
                    && match field.as_str() {
                        "record.heart_rate" => i < 2,
                        "session.total_calories" | "session.metabolic_calories" => i == 0,
                        _ => true,
                    };
                if !allowed {
                    return Err(invalid(&at, "source incompatible with this selection rule"));
                }
            }
        }
        if streams.get("record.position_lat") != streams.get("record.position_long") {
            return Err(invalid(
                "/merge/streams/record.position_long",
                "coordinate source priorities must match",
            ));
        }
        if let (Some(total), Some(metabolic)) = (
            streams.get("session.total_calories"),
            streams.get("session.metabolic_calories"),
        ) && total != metabolic
        {
            return Err(invalid(
                "/merge/streams/session.metabolic_calories",
                "calories must use the same session source",
            ));
        }
        Ok(Self {
            sources,
            targets,
            devices: definitions
                .iter()
                .filter(|(_, v)| v.get("device").is_some())
                .map(|(id, v)| r.device(id, v).map(|device| (id.clone(), device)))
                .collect::<Result<_>>()?,
            record_fields,
            selection: merge,
            timezone: timezone.into(),
            output_dir: resolve(base, v["output_dir"].as_str().unwrap())?,
        })
    }
    pub fn device(&self, id: &str) -> Option<Device> {
        self.devices.get(id).copied()
    }
    pub fn sources(&self) -> impl Iterator<Item = &dyn DataSource> {
        self.sources.values().map(Box::as_ref)
    }
    pub fn source(&self, id: &str) -> Option<&dyn DataSource> {
        self.sources.get(id).map(Box::as_ref)
    }
    pub fn primary_source(&self) -> &dyn DataSource {
        self.sources[self.selection["primary_source"].as_str().unwrap()].as_ref()
    }
    pub fn targets(&self) -> impl Iterator<Item = &Target> {
        self.targets.iter()
    }
    pub(crate) fn record_fields(&self) -> impl Iterator<Item = (&str, u8)> {
        self.record_fields
            .iter()
            .filter(|(name, _)| self.selection["streams"].get(name).is_some())
            .map(|(name, id)| (name.as_str(), *id))
    }
    pub fn selected_fields(&self) -> impl Iterator<Item = &str> {
        self.selection["streams"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
    }
    /// An omitted stream follows the primary device without an explicit override.
    pub fn sources_for(&self, field: &str) -> impl Iterator<Item = &dyn DataSource> {
        let inherited = self.selection["streams"].get(field).is_none()
            && (field.starts_with("record.")
                || matches!(
                    field,
                    "session.total_calories" | "session.metabolic_calories"
                ));
        self.configured_sources(field)
            .chain(inherited.then(|| self.primary_source()))
    }
    pub(crate) fn configured_sources(&self, field: &str) -> impl Iterator<Item = &dyn DataSource> {
        self.selection["streams"][field]
            .as_array()
            .into_iter()
            .flatten()
            .map(|id| self.sources[id.as_str().unwrap()].as_ref())
    }
    pub(crate) fn selection(&self) -> &Value {
        &self.selection
    }
    pub(crate) fn merge_policy(&self) -> MergePolicy<'_> {
        MergePolicy { settings: self }
    }
    pub fn minimum_overlap(&self) -> f64 {
        self.selection["min_overlap_ratio"].as_f64().unwrap()
    }
    pub fn primary_query(&self, day: &str) -> Result<crate::data_source::ActivityQuery> {
        use jiff::civil::Date;
        let date = parse_day(day)?;
        let midnight = |date: Date| {
            date.at(0, 0, 0, 0)
                .in_tz(&self.timezone)
                .map(|z| z.timestamp().as_second())
                .map_err(|e| Error::usage(e.to_string()))
        };
        Ok(crate::data_source::ActivityQuery::StartsWithin {
            start_utc: midnight(date)?,
            end_utc: midnight(date.tomorrow().map_err(|e| Error::usage(e.to_string()))?)?,
        })
    }
    pub fn candidate_query(
        &self,
        start_utc: i64,
        end_utc: i64,
    ) -> crate::data_source::ActivityQuery {
        crate::data_source::ActivityQuery::Overlapping {
            start_utc,
            end_utc,
            minimum_overlap: self.minimum_overlap(),
        }
    }
}
#[cfg(test)]
mod tests;
