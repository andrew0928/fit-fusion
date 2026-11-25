//! Output contract shared by the standalone activity query tools.
use clap::Args;
use jiff::{Timestamp, tz::TimeZone};
use serde_json::Value;

#[derive(Args)]
pub struct Output {
    /// Print compact JSON (default: indented JSON)
    #[arg(long, global = true, conflicts_with = "text")]
    pub json: bool,
    /// Print a human-readable report
    #[arg(long, global = true)]
    pub text: bool,
}

impl Output {
    // clap checks conflicts per command level. Global flags may straddle a subcommand.
    pub fn validate(&self) -> clap::error::Result<()> {
        if self.json && self.text {
            return Err(clap::Error::raw(
                clap::error::ErrorKind::ArgumentConflict,
                "--json and --text cannot be used together",
            ));
        }
        Ok(())
    }
    pub fn print(&self, report: &Value, zone: &TimeZone) {
        if self.text {
            print_text(report, zone);
        } else if self.json {
            println!("{report}");
        } else {
            println!("{report:#}");
        }
    }
}

// Source names, titles and paths may contain newlines or terminal control characters.
fn label(value: &Value) -> String {
    match value {
        Value::String(s) => s
            .chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect(),
        Value::Number(n) => n.to_string(),
        _ => "—".into(),
    }
}

fn local_time(v: &Value, zone: &TimeZone) -> String {
    v.as_str()
        .and_then(|s| s.parse::<Timestamp>().ok())
        .map(|t| {
            t.to_zoned_datetime(zone.clone())
                .strftime("%Y-%m-%d %H:%M:%S %:z")
                .to_string()
        })
        .unwrap_or_else(|| label(v))
}

fn activity(a: &Value, zone: &TimeZone) {
    println!(
        "{}  {}  ·  {}",
        local_time(&a["start_utc"], zone),
        label(&a["activity_name"]),
        label(&a["device_name"])
    );
    println!("  ID       {}", label(&a["id"]));
    println!(
        "  Source   {}",
        label(a.get("source_name").unwrap_or(&a["source_id"]))
    );
    if a["path"].is_string() {
        println!("  File     {}", label(&a["path"]));
    }
}

fn diagnostic(e: &Value) {
    let subject = e.get("path").or_else(|| e.get("code"));
    if let Some(subject) = subject {
        println!("  {}  {}", label(subject), label(&e["message"]));
    } else {
        println!("  {}", label(&e["message"]));
    }
    if e["retry_after"].is_string() {
        println!("  Retry after  {}", label(&e["retry_after"]));
    }
}

fn print_text(report: &Value, zone: &TimeZone) {
    println!("Status  {}", label(&report["status"]));
    if let Some(rows) = report["activities"].as_array() {
        println!(
            "Activities  {}  (departure · activity · device)",
            rows.len()
        );
        for row in rows {
            activity(row, zone);
        }
    }
    if let Some(a) = report.get("activity") {
        activity(a, zone);
        println!("  End      {}", local_time(&a["end_utc"], zone));
    }
    if let Some(streams) = report["streams"].as_array() {
        println!("Streams  {}  (samples · first · last)", streams.len());
        for s in streams {
            println!(
                "  {}  {}  {}  {}",
                label(&s["name"]),
                label(&s["sample_count"]),
                local_time(&s["start_utc"], zone),
                local_time(&s["end_utc"], zone)
            );
        }
    }
    if let Some(items) = report["exports"].as_array() {
        println!("Exports  {}", items.len());
        for item in items {
            println!(
                "  {}  {}  {}",
                label(&item["status"]),
                label(&item["id"]),
                label(&item["path"])
            );
            println!(
                "    {}  ·  {}",
                label(&item["activity_name"]),
                label(&item["device_name"])
            );
            if let Some(e) = item.get("error") {
                diagnostic(e);
            }
        }
    }
    if let Some(errors) = report["errors"].as_array() {
        for e in errors {
            diagnostic(e);
        }
    }
    if let Some(e) = report.get("error") {
        diagnostic(e);
    }
}
