use crate::{
    Error, Result,
    config::Settings,
    native::{NativeValue, Records, read_records},
    output::Calories,
    quality::{self, Selection},
    ride::Ride,
};
use serde_json::json;
use std::collections::BTreeMap;

pub(crate) fn ratios((a, b): (i64, i64), (c, d): (i64, i64)) -> [f64; 2] {
    if b <= a || d <= c {
        return [0.0; 2];
    }
    let intersection = (b.min(d) - a.max(c)).max(0) as f64;
    [intersection / (b - a) as f64, intersection / (d - c) as f64]
}
fn intervals(ride: &Ride) -> [(i64, i64); 2] {
    let (first, last) = ride.span();
    [(first.into(), last.into()), ride.session_utc]
}
pub fn overlap(primary: &Ride, candidate: &Ride) -> [[f64; 2]; 2] {
    let [ar, asess] = intervals(primary);
    let [br, bsess] = intervals(candidate);
    [ratios(ar, br), ratios(asess, bsess)]
}
pub struct MatchedRide {
    pub primary_id: String,
    pub rides: BTreeMap<String, Ride>,
}
impl MatchedRide {
    pub fn primary(&self) -> &Ride {
        &self.rides[&self.primary_id]
    }
}

#[derive(serde::Serialize)]
pub struct SourceRange {
    pub start_fit_seconds: u32,
    pub end_fit_seconds: u32,
    pub source: Option<String>,
}
pub struct FieldSelection {
    pub name: String,
    pub id: u8,
    pub values: Vec<Option<NativeValue>>,
    pub ranges: Vec<SourceRange>,
    pub primary_only: bool,
}
pub struct SelectedRide<'a> {
    pub heart: Selection,
    pub heart_enabled: bool,
    pub heart_source: Option<&'a Ride>,
    pub fields: Vec<FieldSelection>,
    pub calories: Option<Calories>,
    pub calorie_source: Option<&'a Ride>,
    pub write_total: bool,
    pub write_metabolic: bool,
    pub overlaps: BTreeMap<String, [[f64; 2]; 2]>,
}
pub(crate) struct MergePolicy<'a> {
    pub settings: &'a Settings,
}
impl MergePolicy<'_> {
    pub fn select<'a>(&self, pair: &'a MatchedRide) -> Result<SelectedRide<'a>> {
        let primary = pair.primary();
        let mut overlaps = BTreeMap::new();
        for (id, ride) in &pair.rides {
            if id == &pair.primary_id {
                continue;
            }
            let checks = overlap(primary, ride);
            if checks
                .iter()
                .flatten()
                .any(|r| *r < self.settings.minimum_overlap())
            {
                let mut e = Error::new(
                    "unmatched_activities",
                    "riding intervals do not meet the mutual overlap threshold",
                    3,
                );
                e.details = json!({"source":id,"records":checks[0],"sessions":checks[1],"minimum":self.settings.minimum_overlap()});
                return Err(e);
            }
            overlaps.insert(id.clone(), checks);
        }
        let priorities = |field| {
            self.settings
                .configured_sources(field)
                .map(|s| s.id())
                .collect::<Vec<_>>()
        };
        let hr = priorities("record.heart_rate");
        let lookup = |id: &&str| pair.rides.get(*id);
        let heart = quality::select(
            primary,
            hr.first().and_then(lookup),
            hr.get(1).and_then(lookup),
            &hr,
        )?;
        let heart_source = if heart.only_garmin {
            hr.get(1).and_then(lookup)
        } else if heart.only_bryton {
            hr.first().and_then(lookup)
        } else {
            None
        };
        let total = priorities("session.total_calories");
        let metabolic = priorities("session.metabolic_calories");
        let calorie_id = total.first().or(metabolic.first());
        let calorie_source = calorie_id.and_then(lookup);
        let calories = match calorie_id {
            Some(_) => {
                let source = calorie_source.ok_or_else(|| {
                    Error::quality("missing configured session for requested calories")
                })?;
                Some(Calories::read(primary, source, !total.is_empty())?)
            }
            None => None,
        };
        let overrides: Vec<_> = self
            .settings
            .record_fields()
            .filter(|(_, id)| *id != 3)
            .collect();
        let ids = overrides.iter().map(|(_, id)| *id).collect();
        let records = pair
            .rides
            .iter()
            .map(|(id, ride)| {
                Ok((
                    id.as_str(),
                    read_records(&ride.bytes, ride.records.keys(), &ids)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        let fields = overrides
            .into_iter()
            .map(|(name, id)| select_field(pair, &records, name, id, &priorities(name)))
            .collect();
        Ok(SelectedRide {
            heart,
            heart_source,
            heart_enabled: !hr.is_empty(),
            fields,
            calories,
            calorie_source,
            write_total: !total.is_empty(),
            write_metabolic: !metabolic.is_empty(),
            overlaps,
        })
    }
}

fn select_field(
    pair: &MatchedRide,
    records: &BTreeMap<&str, Records>,
    name: &str,
    id: u8,
    order: &[&str],
) -> FieldSelection {
    let primary = pair.primary();
    let sources: Vec<_> = order
        .iter()
        .filter_map(|source| Some((*source, records.get(*source)?)))
        .collect();
    let mut values = Vec::with_capacity(primary.samples.len());
    let mut ranges = Vec::new();
    let mut current: Option<(u32, u32, Option<&str>)> = None;
    let mut primary_only = true;
    let range = |(start, end, source): (u32, u32, Option<&str>)| SourceRange {
        start_fit_seconds: start,
        end_fit_seconds: end,
        source: source.map(str::to_owned),
    };
    for &time in primary.samples.keys() {
        let chosen = sources.iter().find_map(|&(source, ride)| {
            let message = ride.get(&time)?;
            if id <= 1 {
                message.get(&(1 - id))?;
            }
            Some((source, message.get(&id)?))
        });
        let origin = chosen.map(|(source, _)| source);
        let value = chosen.map(|(_, v)| v);
        primary_only &= origin.is_none_or(|s| s == pair.primary_id)
            && value
                == records[pair.primary_id.as_str()]
                    .get(&time)
                    .and_then(|m| m.get(&id));
        if let Some((_, last, previous)) = &mut current
            && *previous == origin
            && last.checked_add(1) == Some(time)
        {
            *last = time;
        } else {
            if let Some(previous) = current {
                ranges.push(range(previous));
            }
            current = Some((time, time, origin));
        }
        values.push(value.cloned());
    }
    if let Some(previous) = current {
        ranges.push(range(previous));
    }
    FieldSelection {
        name: name.to_owned(),
        id,
        values,
        ranges,
        primary_only,
    }
}

#[cfg(test)]
mod tests;
