use crate::{Error, Result, ride::Ride};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub struct Point {
    pub time: u32,
    pub hr: Option<u8>,
    pub source: &'static str,
}
pub struct Selection {
    pub points: Vec<Point>,
    pub report: Value,
    pub only_bryton: bool,
    pub only_garmin: bool,
}
struct Scan {
    triggers: Vec<Value>,
    suspect: Vec<Value>,
    affected: BTreeSet<usize>,
    missing: usize,
    longest_flat: usize,
}
fn scan(
    times: &[u32],
    values: &[Option<u8>],
    other: &[Option<u8>],
    boundaries: &BTreeSet<u32>,
) -> Scan {
    let missing = values.iter().filter(|v| v.is_none()).count();
    let mut s = Scan {
        triggers: vec![],
        suspect: vec![],
        affected: BTreeSet::new(),
        missing,
        longest_flat: 0,
    };
    let excess = missing == values.len() || (missing >= 120 && missing * 100 >= values.len() * 5);
    let mut i = 0;
    while i < values.len() {
        let mut j = i + 1;
        while j < values.len()
            && times[j] - times[j - 1] == 1
            && !boundaries.contains(&times[j])
            && values[j] == values[i]
        {
            j += 1;
        }
        let n = j - i;
        let mut reason = None;
        let mut detail =
            json!({"start_fit_seconds":times[i],"end_fit_seconds":times[j-1],"seconds":n});
        if let Some(v) = values[i] {
            s.longest_flat = s.longest_flat.max(n);
            if n >= 60 {
                let mut bpm: Vec<_> = other[i..j].iter().flatten().copied().collect();
                bpm.sort_unstable();
                let mut run = 0;
                let mut longest = 0;
                for x in &other[i..j] {
                    if x.is_some_and(|x| x.abs_diff(v) >= 5) {
                        run += 1;
                        longest = longest.max(run);
                    } else {
                        run = 0;
                    }
                }
                let spread = if bpm.is_empty() {
                    0
                } else {
                    bpm[(9 * bpm.len()).div_ceil(10) - 1] - bpm[bpm.len().div_ceil(10) - 1]
                };
                detail["bpm"] = json!(v);
                detail["reference_samples"] = json!(bpm.len());
                detail["p90_minus_p10"] = json!(spread);
                detail["longest_divergence_s"] = json!(longest);
                if bpm.len() * 100 >= 95 * n && spread >= 5 && longest >= 20 {
                    reason = Some("stale");
                } else {
                    s.suspect.push(detail.clone());
                }
            }
        } else if n >= 600 {
            reason = Some("long_missing");
        } else if excess {
            reason = Some("missing_coverage");
        }
        if let Some(c) = reason {
            detail["reason"] = json!(c);
            s.triggers.push(detail);
            s.affected.extend(i..j);
        }
        i = j;
    }
    s
}
pub fn select(b: &Ride, g: &Ride) -> Result<Selection> {
    let times: Vec<_> = b.samples.keys().copied().collect();
    // Unexplained recording gaps cannot be treated as adjacent valid seconds.
    for p in times.windows(2) {
        if p[1] - p[0] > 1 && b.boundaries.range(p[0] + 1..=p[1]).next().is_none() {
            return Err(Error::quality(
                "non-1Hz primary timeline without a timer boundary",
            ));
        }
    }
    let primary: Vec<_> = b.samples.values().copied().collect();
    let second: Vec<_> = times
        .iter()
        .map(|t| g.samples.get(t).copied().flatten())
        .collect();
    let boundaries = b.boundaries.union(&g.boundaries).copied().collect();
    let checks = scan(&times, &primary, &second, &boundaries);
    let switch = !checks.triggers.is_empty();
    let secondary = scan(&times, &second, &primary, &boundaries);
    let affected_valid = checks
        .affected
        .iter()
        .filter(|&&i| second[i].is_some())
        .count();
    let n = times.len();
    let mut report = json!({"mode":if switch {"whole_garmin"}else{"bryton_with_gap_fill"},
        "primary_valid":n-checks.missing,"garmin_valid":n-secondary.missing,"record_count":n,
        "triggers":checks.triggers,"plateaus_without_confirmation":checks.suspect,
        "affected_seconds":checks.affected.len(),"garmin_valid_affected":affected_valid,
        "garmin_triggers":secondary.triggers,"garmin_longest_constant_seconds":secondary.longest_flat});
    if switch
        && ((n - secondary.missing) * 100 < n * 95
            || affected_valid * 100 < checks.affected.len() * 95
            || !secondary.triggers.is_empty()
            || secondary.longest_flat >= 600)
    {
        let mut e =
            Error::quality("Garmin coverage or quality insufficient for whole-stream replacement");
        e.details = report;
        return Err(e);
    }
    let mut counts = [0usize; 3];
    let mut points = Vec::with_capacity(n);
    for (i, &time) in times.iter().enumerate() {
        let (hr, source) = if !switch && primary[i].is_some() {
            (primary[i], 0)
        } else if second[i].is_some() {
            (second[i], 1)
        } else {
            (None, 2)
        };
        counts[source] += 1;
        points.push(Point {
            time,
            hr,
            source: ["bryton", "garmin", "missing"][source],
        });
    }
    report["samples"] = json!({"bryton":counts[0],"garmin":counts[1],"missing":counts[2]});
    Ok(Selection {
        points,
        report,
        only_bryton: !switch && counts[1] == 0,
        only_garmin: switch,
    })
}
