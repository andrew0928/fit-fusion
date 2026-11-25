//! Stream the main HealthData XML; large exports also contain an unrelated CDA XML.
use crate::{Result, bad};
use jiff::Timestamp;
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
use zip::ZipArchive;

pub type Archive = ZipArchive<File>;
type Attrs = HashMap<String, String>;

pub fn attr(e: &BytesStart, key: &str) -> Result<Option<String>> {
    Ok(e.try_get_attribute(key)?
        .map(|a| a.unescape_value().map(|v| v.into_owned()))
        .transpose()?)
}
fn attrs(e: &BytesStart) -> Result<Attrs> {
    e.attributes()
        .map(|a| {
            let a = a?;
            Ok((
                std::str::from_utf8(a.key.as_ref())?.into(),
                a.unescape_value()?.into_owned(),
            ))
        })
        .collect()
}
fn need<'a>(a: &'a Attrs, key: &str) -> Result<&'a str> {
    a.get(key)
        .map(String::as_str)
        .ok_or_else(|| bad(format!("missing XML attribute {key}")))
}
fn stamp(s: &str) -> Result<Timestamp> {
    Ok(Timestamp::strptime("%Y-%m-%d %H:%M:%S %z", s).or_else(|_| s.parse::<Timestamp>())?)
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn device_part(device: &str, key: &str) -> Option<String> {
    device
        .split_once(&format!("{key}:"))
        .map(|(_, rest)| {
            rest.split_once(", ")
                .map_or(rest, |(v, _)| v)
                .trim_end_matches('>')
                .trim()
                .to_string()
        })
        .filter(|s| !s.is_empty())
}
fn hardware(device: &str) -> Option<String> {
    device_part(device, "hardware")
}
fn display_device(device: &str) -> Option<String> {
    match hardware(device).as_deref() {
        Some("Watch7,1" | "Watch7,2" | "Watch7,3" | "Watch7,4") => {
            Some("Apple Watch Series 9".into())
        }
        _ => device_part(device, "name").or_else(|| device_part(device, "model")),
    }
}

#[derive(Serialize)]
pub struct Workout {
    pub id: String,
    pub source_id: &'static str,
    pub source_name: String,
    pub device_name: Option<String>,
    pub activity_name: Option<String>,
    pub sport: &'static str,
    pub start_utc: Timestamp,
    pub end_utc: Timestamp,
    pub device: Option<String>,
    #[serde(skip)]
    pub routes: Vec<String>,
    #[serde(skip)]
    pub timer_s: Option<f64>,
    #[serde(skip)]
    pub distance_m: Option<f64>,
    #[serde(skip)]
    pub kcal: Option<f64>,
    #[serde(skip)]
    external: Option<String>,
}
fn quantity(value: &str, unit: &str, kind: &str) -> Result<f64> {
    let factor = match (kind, unit) {
        ("timer", "s")
        | ("distance", "m")
        | ("energy", "kcal" | "Cal")
        | ("hr", "count/min")
        | ("power", "W")
        | ("cadence", "count/min" | "rpm")
        | ("speed", "m/s") => 1.,
        ("timer", "min") => 60.,
        ("timer", "hr" | "h") => 3600.,
        ("distance", "km") => 1000.,
        ("distance", "mi") => 1609.344,
        ("energy", "kJ") => 1. / 4.184,
        ("energy", "cal") => 0.001,
        ("speed", "km/hr" | "km/h") => 1. / 3.6,
        ("speed", "mi/hr" | "mph") => 0.44704,
        _ => return Err(bad(format!("unsupported {kind} unit {unit}"))),
    };
    let n = value.parse::<f64>()? * factor;
    if !n.is_finite() || n < 0. {
        return Err(bad(format!("invalid {kind} value")));
    }
    Ok(n)
}
impl Workout {
    fn from_xml(a: Attrs) -> Result<Self> {
        let source = need(&a, "sourceName")?.to_owned();
        let device = a.get("device").map(String::as_str).unwrap_or("");
        let value = |key: &str, unit: &str, kind: &str| -> Result<Option<f64>> {
            a.get(key)
                .map(|v| quantity(v, need(&a, unit)?, kind))
                .transpose()
        };
        let start = stamp(need(&a, "startDate")?)?;
        let end = stamp(need(&a, "endDate")?)?;
        if end <= start {
            return Err(bad("invalid cycling Workout time range"));
        }
        Ok(Self {
            id: String::new(),
            source_id: "apple_health",
            source_name: source,
            device_name: display_device(device),
            device: hardware(device),
            activity_name: a
                .get("activityName")
                .cloned()
                .filter(|s| !s.trim().is_empty()),
            sport: "cycling",
            start_utc: start,
            end_utc: end,
            timer_s: value("duration", "durationUnit", "timer")?,
            distance_m: value("totalDistance", "totalDistanceUnit", "distance")?,
            kcal: value("totalEnergyBurned", "totalEnergyBurnedUnit", "energy")?,
            routes: vec![],
            external: None,
        })
    }
    fn child(&mut self, e: &BytesStart) -> Result<()> {
        match e.local_name().as_ref() {
            b"FileReference" => {
                if let Some(path) = attr(e, "path")?
                    && path.to_lowercase().ends_with(".gpx")
                {
                    self.routes.push(path);
                }
            }
            b"MetadataEntry" => match attr(e, "key")?.as_deref() {
                Some("HKExternalUUID" | "HKMetadataKeyExternalUUID") => {
                    self.external = attr(e, "value")?.filter(|s| !s.is_empty())
                }
                Some("HKWorkoutActivityName" | "HKMetadataKeyWorkoutActivityName") => {
                    self.activity_name = attr(e, "value")?.filter(|s| !s.trim().is_empty())
                }
                _ => (),
            },
            b"WorkoutStatistics" => {
                if let Some(v) = attr(e, "sum")? {
                    let u = attr(e, "unit")?.unwrap_or_default();
                    match attr(e, "type")?.as_deref() {
                        Some("HKQuantityTypeIdentifierDistanceCycling") => {
                            self.distance_m = Some(quantity(&v, &u, "distance")?)
                        }
                        Some("HKQuantityTypeIdentifierActiveEnergyBurned") => {
                            self.kcal = Some(quantity(&v, &u, "energy")?)
                        }
                        _ => (),
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }
    fn finish(mut self) -> Self {
        let identity = match &self.external {
            Some(uuid) => serde_json::json!([self.source_name, "uuid", uuid]),
            None => serde_json::json!([self.source_name, self.device, self.sport, self.start_utc]),
        };
        self.id = hash(identity.to_string().as_bytes());
        let mut seen = HashSet::new();
        self.routes.retain(|r| seen.insert(r.clone()));
        self
    }
}

fn root_tag<R: BufRead>(input: R) -> Result<Vec<u8>> {
    let mut r = Reader::from_reader(input);
    let mut buf = Vec::new();
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) | Event::Empty(e) => return Ok(e.local_name().into_inner().to_vec()),
            Event::Eof => return Err(bad("empty XML document")),
            _ => (),
        }
        buf.clear();
    }
}
pub fn open(path: &Path) -> Result<(Archive, String)> {
    let mut zip = Archive::new(File::open(path)?)?;
    let xml: Vec<_> = zip
        .file_names()
        .filter(|p| {
            p.ends_with(".xml") && !p.ends_with("/export_cda.xml") && *p != "export_cda.xml"
        })
        .map(str::to_owned)
        .collect();
    let mut names = vec![];
    for name in xml {
        if root_tag(BufReader::new(zip.by_name(&name)?))? == b"HealthData" {
            names.push(name);
        }
    }
    if names.len() != 1 {
        return Err(bad("expected exactly one HealthData XML in the ZIP"));
    }
    Ok((zip, names.pop().unwrap()))
}
pub fn list(zip: &mut Archive, name: &str) -> Result<Vec<Workout>> {
    let mut r = Reader::from_reader(BufReader::new(zip.by_name(name)?));
    let mut buf = vec![];
    let mut current: Option<Workout> = None;
    let mut depth = 0usize;
    let mut workout_depth = 0;
    let mut route_depth = None;
    let mut complete = false;
    let mut result = Vec::new();
    let mut ids = HashSet::new();
    let mut finish = |w: Workout| -> Result<()> {
        let w = w.finish();
        if !ids.insert(w.id.clone()) {
            return Err(bad(format!("ambiguous Workout identity {}", w.id)));
        }
        result.push(w);
        Ok(())
    };
    loop {
        let ev = r.read_event_into(&mut buf)?;
        match &ev {
            Event::Empty(e) if e.local_name().as_ref() == b"HealthData" && depth == 0 => {
                complete = true
            }
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"Workout" => {
                if attr(e, "workoutActivityType")?.as_deref()
                    == Some("HKWorkoutActivityTypeCycling")
                {
                    let w = Workout::from_xml(attrs(e)?)?;
                    if matches!(ev, Event::Empty(_)) {
                        finish(w)?;
                    } else {
                        current = Some(w);
                        workout_depth = depth + 1;
                    }
                }
            }
            Event::Start(e) | Event::Empty(e) => {
                if let Some(w) = &mut current {
                    if depth == workout_depth
                        && e.local_name().as_ref() == b"WorkoutRoute"
                        && matches!(ev, Event::Start(_))
                    {
                        route_depth = Some(depth + 1);
                    }
                    if depth == workout_depth
                        || (route_depth == Some(depth)
                            && e.local_name().as_ref() == b"FileReference")
                    {
                        w.child(e)?;
                    }
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"Workout" => {
                if let Some(w) = current.take() {
                    finish(w)?;
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"WorkoutRoute" => route_depth = None,
            Event::End(e) if e.local_name().as_ref() == b"HealthData" && depth == 1 => {
                complete = true
            }
            Event::Eof => break,
            _ => (),
        }
        match ev {
            Event::Start(_) => depth += 1,
            Event::End(_) => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| bad("unbalanced HealthData XML"))?
            }
            _ => (),
        }
        buf.clear();
    }
    if !complete || depth != 0 || current.is_some() {
        return Err(bad("truncated Workout"));
    }
    result.sort_by(|a, b| (a.start_utc, &a.id).cmp(&(b.start_utc, &b.id)));
    Ok(result)
}

// Each row is one observed time. Complementary observations may share one FIT record.
#[derive(Default)]
pub struct Observations {
    pub rows: BTreeMap<Timestamp, Vec<BTreeMap<Metric, f64>>>,
    pub route_points: usize,
    pub missing_routes: Vec<String>,
    pub error: Option<String>,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Metric {
    Lat,
    Lon,
    Altitude,
    Speed,
    Heart,
    Power,
    Cadence,
    DistanceDelta,
    IntervalMs,
}
impl Observations {
    pub fn add(&mut self, t: Timestamp, values: BTreeMap<Metric, f64>) {
        let slots = self.rows.entry(t).or_default();
        if let Some(slot) = slots
            .iter_mut()
            .find(|s| values.keys().all(|k| !s.contains_key(k)))
        {
            slot.extend(values);
        } else {
            slots.push(values);
        }
    }
}
pub fn samples(zip: &mut Archive, xml: &str, workouts: &[Workout]) -> Result<Vec<Observations>> {
    let mut all: Vec<_> = workouts.iter().map(|_| Observations::default()).collect();
    if workouts.is_empty() {
        return Ok(all);
    }
    let mut sources: HashMap<&str, Vec<(usize, Timestamp)>> = HashMap::new();
    for (i, w) in workouts.iter().enumerate() {
        let windows = sources.entry(&w.source_name).or_default();
        let latest_end = windows.last().map_or(w.end_utc, |(_, t)| w.end_utc.max(*t));
        windows.push((i, latest_end));
    }
    let mut r = Reader::from_reader(BufReader::new(zip.by_name(xml)?));
    let mut buf = vec![];
    loop {
        match r.read_event_into(&mut buf)? {
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"Record" => {
                let kind = attr(&e, "type")?.unwrap_or_default();
                let (metric, unit) = match kind.strip_prefix("HKQuantityTypeIdentifier") {
                    Some("HeartRate") => (Metric::Heart, "hr"),
                    Some("CyclingPower") => (Metric::Power, "power"),
                    Some("CyclingCadence") => (Metric::Cadence, "cadence"),
                    Some("CyclingSpeed") => (Metric::Speed, "speed"),
                    Some("DistanceCycling") => (Metric::DistanceDelta, "distance"),
                    _ => {
                        buf.clear();
                        continue;
                    }
                };
                let source = attr(&e, "sourceName")?.unwrap_or_default();
                if let Some(indices) = sources.get(source.as_str()) {
                    let a = attrs(&e)?;
                    let t = stamp(need(&a, "startDate")?)?;
                    let end = stamp(need(&a, "endDate")?)?;
                    let device = a.get("device").and_then(|s| hardware(s));
                    // Only visit windows that could contain the complete sample interval.
                    let stop = indices.partition_point(|(i, _)| workouts[*i].start_utc <= t);
                    for &(i, _) in indices[..stop]
                        .iter()
                        .rev()
                        .take_while(|(_, latest_end)| *latest_end >= end)
                    {
                        let w = &workouts[i];
                        if t < w.start_utc
                            || end > w.end_utc
                            || end < t
                            || device
                                .as_ref()
                                .zip(w.device.as_ref())
                                .is_some_and(|(a, b)| a != b)
                        {
                            continue;
                        }
                        let val = match quantity(need(&a, "value")?, need(&a, "unit")?, unit) {
                            Ok(v) => v,
                            Err(e) => {
                                all[i].error = Some(e.to_string());
                                continue;
                            }
                        };
                        let mut fields = BTreeMap::from([(metric, val)]);
                        if end > t {
                            fields.insert(
                                Metric::IntervalMs,
                                (end.as_millisecond() - t.as_millisecond()) as f64,
                            );
                        }
                        all[i].add(t, fields);
                    }
                }
            }
            Event::Eof => break,
            _ => (),
        }
        buf.clear();
    }
    drop(r);
    let parent = xml.rsplit_once('/').map_or("", |(p, _)| p);
    for (w, out) in workouts.iter().zip(&mut all) {
        for reference in &w.routes {
            let relative = reference.trim_start_matches('/');
            if relative.split('/').any(|s| matches!(s, "" | ".." | ".")) {
                out.error = Some("invalid GPX member path".into());
                continue;
            }
            let name = if parent.is_empty() {
                relative.to_string()
            } else {
                format!("{parent}/{relative}")
            };
            match zip.by_name(&name) {
                Ok(file) => {
                    if let Err(e) = crate::route::read(BufReader::new(file), w, out) {
                        out.error = Some(e.to_string());
                    }
                }
                Err(zip::result::ZipError::FileNotFound) => {
                    out.missing_routes.push(reference.clone())
                }
                Err(e) => return Err(e.into()),
            }
        }
    }
    Ok(all)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantity_converts_known_units_without_guessing() {
        assert_eq!(quantity("36", "km/h", "speed").unwrap(), 10.);
        assert_eq!(quantity("1", "mi", "distance").unwrap(), 1609.344);
        for (value, unit) in [("NaN", "W"), ("-1", "W"), ("10", "kcal")] {
            assert!(quantity(value, unit, "power").is_err());
        }
    }

    #[test]
    fn same_instant_preserves_two_observations_of_the_same_metric() {
        let time = stamp("2026-11-21 14:00:00 +0800").unwrap();
        let mut obs = Observations::default();
        for (metric, v) in [
            (Metric::Heart, 120.),
            (Metric::Power, 0.),
            (Metric::Heart, 122.),
        ] {
            obs.add(time, BTreeMap::from([(metric, v)]));
        }
        let slots = &obs.rows[&time];
        assert_eq!(slots.len(), 2);
        assert_eq!(slots[0][&Metric::Power], 0.);
        assert_eq!(slots[1][&Metric::Heart], 122.);
    }
}
