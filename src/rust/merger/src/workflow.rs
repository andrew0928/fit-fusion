//! Resolve configured sources first, then apply the same selection policy for each ride.
use crate::{
    Error, ExportFormat, Result,
    config::Settings,
    data_source::{ActivitySummary, DataSource, Device, FitDirectory},
    policy::{MatchedRide, overlap},
    ride::Ride,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

pub(crate) fn required(settings: &Settings) -> BTreeSet<String> {
    settings
        .selected_fields()
        .flat_map(|f| settings.sources_for(f))
        .map(DataSource::id)
        .filter(|id| *id != settings.primary_source().id())
        .map(str::to_owned)
        .collect()
}
struct Candidate {
    ride: Ride,
    detail: Value,
}
struct Plan {
    primary: Ride,
    candidates: BTreeMap<String, Vec<Candidate>>,
    diagnostics: BTreeMap<String, Vec<Value>>,
    failure: Option<Error>,
}
fn candidates(
    settings: &Settings,
    source: &dyn DataSource,
    primary: &Ride,
    start: i64,
    end: i64,
    known: Option<&str>,
) -> Result<(Vec<Candidate>, Vec<Value>)> {
    let ids = if let Some(id) = known {
        vec![id.into()]
    } else {
        source
            .list_activities(&settings.candidate_query(start, end))?
            .into_iter()
            .map(|a| a.activity_id)
            .collect()
    };
    let mut found = Vec::new();
    let mut examined = Vec::new();
    let mut seen_ids = BTreeSet::new();
    let mut seen_hash = BTreeSet::new();
    for id in ids {
        if !seen_ids.insert(id.clone()) {
            continue;
        }
        let r = match source.open_activity(&id) {
            Err(e) if e.code == "device_mismatch" && known.is_none() => continue,
            r => r?,
        };
        let meta = r.metadata().clone();
        let ride = Ride::from_recording(r)?;
        if ride.hash == primary.hash || !seen_hash.insert(ride.hash.clone()) {
            continue;
        }
        let ratios = overlap(primary, &ride);
        let detail = json!({"source_id":source.id(),"activity_id":id,"sha256":ride.hash,"start_utc":meta.start_utc,"end_utc":meta.end_utc,
            "record_ratios":ratios[0],"session_ratios":ratios[1]});
        examined.push(detail.clone());
        if ratios
            .iter()
            .flatten()
            .all(|r| *r >= settings.minimum_overlap())
        {
            found.push(Candidate { ride, detail });
        }
    }
    Ok((found, examined))
}
fn prepare(
    settings: &Settings,
    primary: crate::data_source::ActivityRecording,
    known: Option<&str>,
) -> Result<Plan> {
    let (start, end) = (primary.metadata().start_utc, primary.metadata().end_utc);
    let mut plan = Plan {
        primary: Ride::from_recording(primary)?,
        candidates: BTreeMap::new(),
        diagnostics: BTreeMap::new(),
        failure: None,
    };
    for id in required(settings) {
        let source = settings.source(&id).unwrap();
        let forced = known.filter(|_| {
            settings
                .device(&id)
                .is_some_and(|d| d.manufacturer == Device::CIRQA.manufacturer)
        });
        match candidates(settings, source, &plan.primary, start, end, forced) {
            Ok((fits, details)) => {
                if forced.is_some() && fits.is_empty() {
                    plan.failure = Some(Error::new(
                        "unmatched_activities",
                        "specified activity fails mutual overlap",
                        3,
                    ));
                }
                plan.diagnostics.insert(id.clone(), details);
                plan.candidates.insert(id, fits);
            }
            Err(e) => {
                plan.failure = Some(e);
                break;
            }
        }
    }
    Ok(plan)
}
type Owners = BTreeMap<(String, String), usize>;
fn owners<'a>(plans: impl Iterator<Item = &'a Plan>) -> Owners {
    let mut map = BTreeMap::new();
    for plan in plans {
        for (id, items) in &plan.candidates {
            for c in items {
                *map.entry((id.clone(), c.ride.hash.clone())).or_default() += 1;
            }
        }
    }
    map
}
fn finish(
    settings: &Settings,
    plan: Plan,
    owners: &Owners,
    out: &Path,
    date_mode: bool,
    format: ExportFormat,
) -> Result<Value> {
    if let Some(mut e) = plan.failure {
        if e.details.is_null() {
            e.details = json!({"examined":plan.diagnostics});
        }
        return Err(e);
    }
    let mut rides = BTreeMap::new();
    let primary_id = settings.primary_source().id().to_owned();
    rides.insert(primary_id.clone(), plan.primary);
    let mut matched = BTreeMap::new();
    for (id, mut items) in plan.candidates {
        if items.len() > 1
            || items.iter().any(|c| {
                owners
                    .get(&(id.clone(), c.ride.hash.clone()))
                    .is_some_and(|n| *n > 1)
            })
        {
            let mut e = Error::new(
                "ambiguous_match",
                "multiple qualifying pairings for this source. Review the candidate activity IDs and times",
                3,
            );
            e.details = json!({"source_id":id,"candidates":items.iter().map(|c|&c.detail).collect::<Vec<_>>()});
            return Err(e);
        }
        if let Some(c) = items.pop() {
            matched.insert(id.clone(), c.detail);
            rides.insert(id, c.ride);
        }
    }
    crate::export(
        settings,
        &MatchedRide { primary_id, rides },
        out,
        date_mode,
        format,
    )
    .map(|mut result| {
        result["matched_sources"] = json!(matched);
        result
    })
    .map_err(|mut e| {
        if !e.details.is_object() {
            e.details = json!({});
        }
        e.details["examined"] = json!(plan.diagnostics);
        e
    })
}
pub fn date(settings: &Settings, day: &str) -> Result<(Value, u8)> {
    date_with_format(settings, day, ExportFormat::Fit)
}
pub fn date_with_format(
    settings: &Settings,
    day: &str,
    format: ExportFormat,
) -> Result<(Value, u8)> {
    let query = settings.primary_query(day)?;
    let primary = settings.primary_source();
    let mut rows = primary.list_activities(&query)?;
    rows.sort_by(|a, b| (a.start_utc, &a.activity_id).cmp(&(b.start_utc, &b.activity_id)));
    let mut plans = Vec::new();
    let mut seen = BTreeSet::new();
    for row in rows {
        if !seen.insert(row.activity_id.clone()) {
            continue;
        }
        let plan = primary
            .open_activity(&row.activity_id)
            .and_then(|p| prepare(settings, p, None));
        plans.push((row, plan));
    }
    let used = owners(plans.iter().filter_map(|(_, r)| r.as_ref().ok()));
    let mut exit = 0;
    let mut rides = Vec::new();
    let mut succeeded = 0;
    for (
        ActivitySummary {
            activity_id,
            start_utc,
            ..
        },
        plan,
    ) in plans
    {
        let mut result = match plan.and_then(|p| {
            finish(
                settings,
                p,
                &used,
                &settings.output_dir.join(day),
                true,
                format,
            )
        }) {
            Ok(result) => {
                succeeded += 1;
                result
            }
            Err(e) => {
                exit = exit.max(e.exit);
                json!({"status":"error","code":e.code,"message":e.message,"details":e.details,"artifacts":{}})
            }
        };
        result["activity_id"] = json!(activity_id);
        result["start_utc"] = json!(start_utc);
        rides.push(result);
    }
    let status = if rides.is_empty() {
        "no_activities"
    } else if exit == 0 {
        "exported"
    } else if succeeded > 0 {
        "partial"
    } else {
        "error"
    };
    let code = if status == "error" {
        rides
            .iter()
            .find_map(|r| r["code"].as_str())
            .unwrap_or(status)
    } else {
        status
    };
    Ok((
        json!({"schema_version":1,"status":status,"code":code,"date":day,"timezone":settings.timezone,"rides":rides,"artifacts":{},"next_action":null}),
        exit,
    ))
}
pub fn manual(
    settings: &Settings,
    inputs: &[PathBuf],
    auto: bool,
    known: Option<&str>,
    out: &Path,
) -> Result<Value> {
    manual_with_format(settings, inputs, auto, known, out, ExportFormat::Fit)
}
pub fn manual_with_format(
    settings: &Settings,
    inputs: &[PathBuf],
    auto: bool,
    known: Option<&str>,
    out: &Path,
    format: ExportFormat,
) -> Result<Value> {
    match inputs {
        [a, b] if !auto && known.is_none() => crate::manual_pair(
            settings,
            FitDirectory::open_file(a)?,
            FitDirectory::open_file(b)?,
            out,
            format,
        ),
        [a] if auto || known.is_some() => {
            let primary = FitDirectory::open_file(a)?;
            if settings
                .device(settings.primary_source().id())
                .is_some_and(|d| d != primary.metadata().device)
            {
                return Err(Error::data("expected the configured primary device"));
            }
            if known.is_some()
                && required(settings)
                    .iter()
                    .filter(|id| {
                        settings
                            .device(id)
                            .is_some_and(|d| d.manufacturer == Device::CIRQA.manufacturer)
                    })
                    .count()
                    != 1
            {
                return Err(Error::usage(
                    "--garmin-id requires one configured Garmin source in the selection",
                ));
            }
            let plan = prepare(settings, primary, known)?;
            finish(settings, plan, &Owners::new(), out, false, format)
        }
        _ => Err(Error::usage(
            "provide two FITs or one primary FIT with --auto-match or --garmin-id ID",
        )),
    }
}
