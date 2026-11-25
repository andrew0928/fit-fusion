use crate::wire::{self, Field, Layout, Record, TYPES, decoded};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::Result;
use time::OffsetDateTime;

#[derive(Default)]
struct Timing {
    count: usize,
    first: Option<i64>,
    last: Option<i64>,
    prev: Option<i64>,
    duplicates: usize,
    backwards: usize,
    intervals: BTreeMap<i64, usize>,
}
impl Timing {
    fn add(&mut self, t: Option<i64>) {
        let Some(t) = t else {
            return;
        };
        self.count += 1;
        self.first = Some(self.first.unwrap_or(t).min(t));
        self.last = Some(self.last.unwrap_or(t).max(t));
        if let Some(p) = self.prev {
            match t - p {
                0 => self.duplicates += 1,
                d if d < 0 => self.backwards += 1,
                d => *self.intervals.entry(d).or_default() += 1,
            }
        }
        self.prev = Some(t);
    }
    fn utc(t: i64) -> String {
        let d =
            OffsetDateTime::from_unix_timestamp(631065600 + t).expect("FIT uint32 date in range");
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            d.year(),
            d.month() as u8,
            d.day(),
            d.hour(),
            d.minute(),
            d.second()
        )
    }
    fn report(&self) -> Value {
        let n: usize = self.intervals.values().sum();
        let at = |mut index: usize| {
            for (&v, &freq) in &self.intervals {
                if index < freq {
                    return v;
                }
                index -= freq;
            }
            unreachable!("median index in histogram")
        };
        let median = (n > 0).then(|| (at((n - 1) / 2) + at(n / 2)) as f64 / 2.0);
        json!({"timestamped_count":self.count,"start_utc":self.first.map(Self::utc),"end_utc":self.last.map(Self::utc),
            "span_s":self.first.zip(self.last).map(|(a,b)| b-a), "min_interval_s":self.intervals.keys().next(),
            "max_interval_s":self.intervals.keys().next_back(),"median_interval_s":median,"typical_hz":median.map(|m|1.0/m),
            "duplicate_intervals":self.duplicates,"backward_intervals":self.backwards})
    }
}

#[derive(Clone, PartialEq, Eq)]
struct Label {
    name: String,
    units: Option<String>,
    base: u8,
    unknown: bool,
    developer: Option<u8>,
    id: u8,
}
#[derive(Default)]
struct Message {
    timing: Timing,
    rows: usize,
    fields: Vec<(Label, BTreeSet<u8>, Timing, usize)>,
}

use wire::{native, timestamp};

#[derive(Default)]
pub struct Developers(HashMap<(u8, u8), DeveloperField>);
struct DeveloperField {
    name: Option<String>,
    units: Option<String>,
    base: u8,
}
impl Developers {
    pub fn observe(&mut self, layout: &Layout, values: &[&[u8]]) {
        if layout.number != 206 {
            return;
        }
        let byte = |id| native(layout, values, id)?.as_u64()?.try_into().ok();
        if let (Some(index), Some(id), Some(base)) = (byte(0), byte(1), byte(2)) {
            let string = |id| {
                native(layout, values, id)
                    .and_then(|v| v.as_str().filter(|s| !s.is_empty()).map(str::to_string))
            };
            self.0.insert(
                (index, id),
                DeveloperField {
                    name: string(3),
                    units: string(8),
                    base,
                },
            );
        }
    }
    pub fn base(&self, f: &Field) -> Option<u8> {
        f.base
            .or_else(|| self.0.get(&(f.developer?, f.id)).map(|d| d.base))
    }
}

fn populated(v: &Value, k: u8) -> bool {
    if v.is_null() {
        return false;
    }
    if let Some(array) = v.as_array() {
        return array.iter().any(|e| populated(e, k));
    }
    match k {
        0 | 2 | 13 => v.as_u64() != Some(255),
        1 => v.as_i64() != Some(127),
        3 => v.as_i64() != Some(32767),
        4 => v.as_u64() != Some(65535),
        5 => v.as_i64() != Some(i32::MAX.into()),
        6 => v.as_u64() != Some(u32::MAX.into()),
        7 => v.as_str() != Some(""),
        8 | 9 => v.as_str() != Some("nan"),
        10 | 11 | 12 | 16 => v.as_u64() != Some(0),
        14 => v.as_i64() != Some(i64::MAX),
        15 => v.as_u64() != Some(u64::MAX),
        _ => true,
    }
}

pub fn inspect(input: &[u8], file: &str, use_sdk: bool) -> Result<Value> {
    let mut names = HashMap::new();
    let mut labels = HashMap::new();
    for line in include_str!("../../../python/fit_fusion/profile.tsv")
        .lines()
        .filter(|s| !s.starts_with('#') && !s.is_empty())
    {
        let cols: Vec<_> = line.split('\t').collect();
        let n = cols[0].parse::<u16>().unwrap();
        if cols[2].is_empty() {
            names.insert(n, cols[1]);
        } else {
            labels.insert((n, cols[2].parse::<u8>().unwrap()), (cols[3], cols[4]));
        }
    }
    let mut members = Vec::new();
    let mut messages = BTreeMap::<u16, Message>::new();
    let mut bindings: [Vec<usize>; 16] = Default::default();
    let mut versions = (0, 0);
    let mut clock = None;
    let mut dev = Developers::default();
    let mut sdk_rows = None;
    wire::walk(input, |frame| {
        match frame.record {
            Record::Header {
                protocol,
                profile,
                size,
            } => {
                if use_sdk {
                    let end = frame.at + frame.bytes.len() + size as usize + 2;
                    sdk_rows = Some(crate::sdk::messages(&input[frame.at..end])?);
                }
                versions = (protocol, profile);
                clock = None;
                dev = Developers::default();
            }
            Record::Definition { local, layout } => {
                let msg = messages.entry(layout.number).or_default();
                bindings[local].clear();
                for f in &layout.fields {
                    let base = dev.base(f).unwrap_or(13);
                    let (name, units) = match f.developer {
                        None => labels
                            .get(&(layout.number, f.id))
                            .map(|&(n, u)| (Some(n), (!u.is_empty()).then_some(u)))
                            .unwrap_or_default(),
                        Some(i) => dev
                            .0
                            .get(&(i, f.id))
                            .map(|d| (d.name.as_deref(), d.units.as_deref()))
                            .unwrap_or_default(),
                    };
                    let label = Label {
                        name: name
                            .map(str::to_owned)
                            .unwrap_or_else(|| format!("unknown_{}", f.id)),
                        units: units.map(str::to_owned),
                        base,
                        unknown: name.is_none(),
                        developer: f.developer,
                        id: f.id,
                    };
                    let index = msg
                        .fields
                        .iter()
                        .position(|(l, ..)| l == &label)
                        .unwrap_or_else(|| {
                            msg.fields
                                .push((label, BTreeSet::new(), Timing::default(), 0));
                            msg.fields.len() - 1
                        });
                    msg.fields[index].1.insert(f.size);
                    bindings[local].push(index);
                }
            }
            Record::Data {
                local,
                layout,
                values,
                offset,
            } => {
                let (t, sdk_values) = if let Some(rows) = &mut sdk_rows {
                    let row = rows
                        .next()
                        .ok_or_else(|| wire::invalid("SDK member missing data message"))?;
                    let (t, parsed) = crate::sdk::observations(&row, layout, offset.is_some())?;
                    (t, Some(parsed))
                } else {
                    (timestamp(layout, &values, offset, &mut clock), None)
                };
                let t = t.filter(|v| (0x10000000..0xffffffff).contains(v));
                let msg = messages.get_mut(&layout.number).unwrap();
                msg.rows += 1;
                msg.timing.add(t);
                let mut seen = HashSet::new();
                for (i, &index) in bindings[local].iter().enumerate() {
                    let (l, _, timing, count) = &mut msg.fields[index];
                    let observed;
                    let value = if let Some(parsed) = &sdk_values {
                        &parsed[i]
                    } else {
                        observed = decoded(values[i], Some(l.base), layout.big);
                        &observed
                    };
                    if populated(value, l.base & 31) && seen.insert(index) {
                        *count += 1;
                        timing.add(t);
                    }
                }
                dev.observe(layout, &values);
            }
            Record::Crc(_) => {
                if let Some(rows) = &mut sdk_rows
                    && rows.next().is_some()
                {
                    return Err(wire::invalid("SDK has additional messages"));
                }
                let rows: Vec<_> = std::mem::take(&mut messages).into_iter().map(|(number,msg)| {
                    let fields:Vec<_>=msg.fields.iter().map(|(l,sizes,t,count)| {
                        let typename=TYPES.get((l.base&31) as usize).map(|s|s.to_string()).unwrap_or_else(||format!("unknown_{}",l.base));
                        json!({"number":l.id,"name":l.name,"units":l.units,"unknown":l.unknown,"base_type":typename,"developer_data_index":l.developer,
                            "encoded_sizes_bytes":sizes,"sample_count":count,"coverage_pct":(msg.rows>0).then(||100.0 * *count as f64 / msg.rows as f64),"timing":t.report()})
                    }).collect();
                    json!({"number":number,"name":names.get(&number).map(|s|s.to_string()).unwrap_or_else(||format!("unknown_{number}")),
                        "unknown":!names.contains_key(&number),"message_count":msg.rows,"fields":fields,"timing":msg.timing.report()})
                }).collect();
                members.push(json!({"protocol_version":format!("{}.{}",versions.0>>4,versions.0&15),
                    "profile_version":format!("{}.{:02}",versions.1/100,versions.1%100),"messages":rows}));
            }
        }
        Ok(())
    })?;
    Ok(json!({"file":file,"members":members}))
}
