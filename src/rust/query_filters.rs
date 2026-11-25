//! Local departure-date and FIT recorder selection for the standalone query CLIs.
use clap::Args;
use fit_inspector::activity::Device;
use jiff::{civil::Date, tz::TimeZone};
use std::ops::Range;

#[derive(Args, Default)]
pub struct Filters {
    /// Local departure date (YYYY-MM-DD)
    #[arg(long, value_parser=parse_date, conflicts_with_all=["from","until"])]
    pub date: Option<Date>,
    /// First departure date to include
    #[arg(long, value_parser=parse_date, requires="until")]
    pub from: Option<Date>,
    /// First departure date to exclude
    #[arg(long, value_parser=parse_date, requires="from")]
    pub until: Option<Date>,
    /// Filter the recording device by numeric FIT IDs, e.g. 267/2205
    #[arg(long, value_parser=parse_device, value_name="MANUFACTURER/PRODUCT")]
    pub device: Option<Device>,
}
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
fn parse_device(text: &str) -> Result<Device, String> {
    let error = "expected numeric FIT manufacturer/product, e.g. 267/2205";
    let (maker, product) = text.split_once('/').ok_or(error)?;
    Ok(Device {
        manufacturer: maker.parse().map_err(|_| error)?,
        product: product.parse().map_err(|_| error)?,
    })
}
impl Filters {
    pub fn days(&self) -> Result<Option<Range<Date>>, String> {
        let days = if let Some(date) = self.date {
            Some(date..date.tomorrow().map_err(|e| e.to_string())?)
        } else if let (Some(from), Some(until)) = (self.from, self.until) {
            if until <= from {
                return Err("--until must be after --from".into());
            }
            Some(from..until)
        } else {
            None
        };
        Ok(days)
    }
}
