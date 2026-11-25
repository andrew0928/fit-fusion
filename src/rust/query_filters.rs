//! Local departure-date and FIT recorder selection for the standalone query CLIs.
use clap::Args;
use fit_inspector::activity::Device;
use jiff::civil::Date;
#[path = "query_dates.rs"]
mod dates;
pub use dates::{parse_date, parse_zone};
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
        dates::days(self.date, self.from, self.until)
    }
}
