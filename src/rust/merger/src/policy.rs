use crate::{
    Error, Result,
    data_source::{ActivityRecording, Device},
    output::Calories,
    quality::{self, Selection},
    ride::Ride,
};
use serde_json::json;
fn overlap((a, b): (u32, u32), (c, d): (u32, u32)) -> [f64; 2] {
    let span = b.min(d).saturating_sub(a.max(c)) as f64;
    [span / (b - a) as f64, span / (d - c) as f64]
}
fn session_span(r: &Ride) -> Result<(u32, u32)> {
    let a = crate::data_source::number(&r.session, 2)
        .ok_or_else(|| Error::quality("session start unavailable"))?;
    let ms = crate::data_source::number(&r.session, 7)
        .ok_or_else(|| Error::quality("session elapsed time unavailable"))?;
    let end = a
        .checked_add(ms.div_ceil(1000))
        .filter(|&end| end > a)
        .ok_or_else(|| Error::quality("invalid session duration"))?;
    Ok((a, end))
}

pub(crate) struct MatchedRide {
    pub primary: Ride,
    pub secondary: Ride,
}
impl MatchedRide {
    pub fn new(first: ActivityRecording, second: ActivityRecording) -> Result<Self> {
        let a = Ride::from_recording(first)?;
        let b = Ride::from_recording(second)?;
        let (primary, secondary) = match (a.device, b.device) {
            (Device::RIDER_460, Device::CIRQA) => (a, b),
            (Device::CIRQA, Device::RIDER_460) => (b, a),
            _ => return Err(Error::data("expected one Rider 460 and one CIRQA")),
        };
        Ok(Self { primary, secondary })
    }
}

pub(crate) struct SelectedRide {
    pub heart: Selection,
    pub calories: Calories,
    pub record_overlap: [f64; 2],
    pub session_overlap: [f64; 2],
}

pub(crate) struct MergePolicy {
    pub minimum_overlap: f64,
}
impl MergePolicy {
    pub fn select(&self, pair: &MatchedRide) -> Result<SelectedRide> {
        let (b, g) = (&pair.primary, &pair.secondary);
        let threshold = self.minimum_overlap;
        let overlap_records = overlap(b.span(), g.span());
        let overlap_sessions = overlap(session_span(b)?, session_span(g)?);
        if overlap_records
            .iter()
            .chain(&overlap_sessions)
            .any(|&r| r < threshold)
        {
            let mut e = Error::new(
                "unmatched_activities",
                "riding intervals do not meet the mutual overlap threshold",
                3,
            );
            e.details =
                json!({"records":overlap_records,"sessions":overlap_sessions,"minimum":threshold});
            return Err(e);
        }

        Ok(SelectedRide {
            heart: quality::select(b, g)?,
            calories: Calories::read(b, g)?,
            record_overlap: overlap_records,
            session_overlap: overlap_sessions,
        })
    }
}
