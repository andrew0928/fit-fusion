use jiff::{civil::Date, tz::TimeZone};
use std::ops::Range;
pub fn parse_date(text: &str) -> Result<Date, String> {
    let date: Date = text.parse().map_err(|_| "expected YYYY-MM-DD")?;
    if date.to_string() != text {
        return Err("expected YYYY-MM-DD".into());
    }
    Ok(date)
}
pub fn parse_zone(text: &str) -> Result<TimeZone, String> {
    TimeZone::get(text).map_err(|e| e.to_string())
}
pub fn days(
    date: Option<Date>,
    from: Option<Date>,
    until: Option<Date>,
) -> Result<Option<Range<Date>>, String> {
    let days = if let Some(date) = date {
        Some(date..date.tomorrow().map_err(|e| e.to_string())?)
    } else if let (Some(from), Some(until)) = (from, until) {
        if until <= from {
            return Err("--until must be after --from".into());
        }
        Some(from..until)
    } else {
        None
    };
    Ok(days)
}
