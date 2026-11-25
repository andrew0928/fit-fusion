//! Derive per-member coverage from the chosen backend's own field counts.
use serde_json::{Value, json};
use std::collections::HashSet;

pub fn selector(s: &str) -> Result<String, String> {
    if let Some((message, field)) = s.split_once('.')
        && [message, field]
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_alphanumeric() || c == '_'))
    {
        return Ok(s.to_owned());
    }
    Err("expected MESSAGE.FIELD, e.g. record.heart_rate".into())
}

fn row(name: &str, total: u64, field: Option<&Value>) -> Value {
    let valid = field.and_then(|f| f["sample_count"].as_u64()).unwrap_or(0);
    let missing = total - valid;
    let state = match (total, field, valid, missing) {
        (0, _, _, _) => "no_messages",
        (_, None, _, _) => "absent",
        (_, _, 0, _) => "no_valid_samples",
        (_, _, _, 0) => "complete",
        _ => "partial",
    };
    json!({"name":name,"message_count":total,"sample_count":valid,"missing_count":missing,
        "missing_pct":(total>0).then(||100.0*missing as f64/total as f64),"state":state,
        "number":field.map(|f| &f["number"]), "developer_data_index":field.map(|f| &f["developer_data_index"])})
}

pub fn annotate(report: &mut Value, streams: &[String]) {
    for member in report["members"].as_array_mut().expect("inventory members") {
        let mut rows = Vec::new();
        let mut found = HashSet::new();
        let mut totals = std::collections::HashMap::new();
        for msg in member["messages"].as_array().expect("inventory messages") {
            let message = msg["name"].as_str().unwrap();
            let total = msg["message_count"].as_u64().unwrap();
            totals.insert(message, total);
            for field in msg["fields"].as_array().unwrap() {
                let name = format!("{message}.{}", field["name"].as_str().unwrap());
                if streams.is_empty() || streams.contains(&name) {
                    rows.push(row(&name, total, Some(field)));
                    found.insert(name);
                }
            }
        }
        for name in streams {
            if found.insert(name.clone()) {
                let (message, _) = name.split_once('.').expect("validated stream");
                rows.push(row(name, totals.get(message).copied().unwrap_or(0), None));
            }
        }
        member["sampling"] = Value::Array(rows);
    }
}
