use super::*;
use serde_json::json;

fn example() -> Value {
    serde_json::from_str(EXAMPLE).unwrap()
}
fn load(v: &Value, r: &Registry) -> Result<Settings> {
    let file = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(file.path(), v.to_string()).unwrap();
    let result = Settings::load_with(file.path(), r);
    assert_eq!(
        crate::data_source::SOURCE_READS.with(|n| n.get()),
        0,
        "A: config opened a source"
    );
    assert_eq!(
        crate::data_source::PLATFORM_REQUESTS.with(|n| n.get()),
        0,
        "A: config made a platform request"
    );
    result
}
fn priority<'a>(s: &'a Settings, field: &str) -> Vec<&'a str> {
    s.sources_for(field).map(DataSource::id).collect()
}
fn issue(v: &Value) -> String {
    let e = load(v, &Registry::default())
        .err()
        .expect("expected config error");
    assert_eq!(e.code, "invalid_config");
    e.details["path"].as_str().unwrap().into()
}

#[test]
fn sources_and_field_selection() {
    let r = Registry::default();
    let mut v = example();
    let s = load(&v, &r).unwrap();
    assert_eq!(
        s.sources().map(DataSource::id).collect::<Vec<_>>(),
        ["apple_watch", "bryton", "garmin"]
    );
    assert_eq!(s.primary_source().id(), "bryton");
    assert_eq!(priority(&s, "record.heart_rate"), ["bryton", "garmin"]);
    assert_eq!(priority(&s, "session.total_calories"), ["garmin"]);
    v["sources"]["garmin"] =
        json!({"kind":"fit_directory","device":"garmin_cirqa","directory":"garmin"});
    let local = load(&v, &r).unwrap();
    assert_eq!(
        local.source("garmin").unwrap().kind(),
        SourceKind::FitDirectory
    );
    for f in s.selected_fields() {
        assert_eq!(priority(&s, f), priority(&local, f));
    }
    let garmin = v["sources"]
        .as_object_mut()
        .unwrap()
        .remove("garmin")
        .unwrap();
    v["sources"]["cirqa"] = garmin;
    for ids in v["merge"]["streams"].as_object_mut().unwrap().values_mut() {
        for id in ids.as_array_mut().unwrap() {
            if id == "garmin" {
                *id = json!("cirqa");
            }
        }
    }
    assert_eq!(
        priority(&load(&v, &r).unwrap(), "record.heart_rate"),
        ["bryton", "cirqa"]
    );
    assert_eq!(
        load(&v, &r).unwrap().device("cirqa"),
        Some(Device::CIRQA),
        "A5a"
    );
    v["merge"]["streams"]["record.heart_rate"][1] = json!("garmin");
    assert_eq!(issue(&v), "/merge/streams/record.heart_rate/1");
    v["sources"] = json!({"bryton":v["sources"]["bryton"]});
    v["merge"]["streams"] = json!({"record.cadence":["bryton"]});
    let only = load(&v, &r).unwrap();
    assert_eq!(only.sources().count(), 1);
    assert_eq!(priority(&only, "record.cadence"), ["bryton"], "A1c");
    assert_eq!(
        only.selected_fields().collect::<Vec<_>>(),
        ["record.cadence"]
    );
    assert!(priority(&only, "record.temperature").is_empty());
}

#[test]
fn multiple_devices_and_new_source_kinds() {
    let mut v = example();
    v["sources"]["old_bike"] =
        json!({"kind":"fit_directory","device":"bryton_neo15","directory":"old"});
    assert_eq!(issue(&v), "/sources/old_bike/device");
    let mut r = Registry::default();
    // Synthetic identity for the test; hardware identification awaits a real Neo15 FIT.
    r.register_device(
        "bryton_neo15",
        Device {
            manufacturer: 267,
            product: 100,
        },
    );
    assert_eq!(load(&v, &r).unwrap().sources().count(), 4);
    v["merge"]["streams"]["record.cadence"] = json!(["bryton", "old_bike"]);
    v["sources"]["archive2"] = json!({"kind":"test_archive","options":{}});
    assert_eq!(
        load(&v, &r).err().unwrap().details["path"],
        "/sources/archive2/kind"
    );
    r.register_source("test_archive", |id, _, _, _| {
        Ok(Box::new(DeferredSource {
            id: id.into(),
            kind: SourceKind::Extension("test_archive"),
        }))
    });
    let s = load(&v, &r).unwrap();
    assert_eq!(s.sources().count(), 5);
    assert_eq!(priority(&s, "record.cadence"), ["bryton", "old_bike"]);
    assert_eq!(priority(&s, "record.heart_rate"), ["bryton", "garmin"]);
    assert_eq!(priority(&s, "session.total_calories"), ["garmin"]);
}

#[test]
fn target_registration_is_independent_of_readers() {
    let mut v = example();
    let mut r = Registry::default();
    v.as_object_mut().unwrap().remove("destinations");
    assert_eq!(load(&v, &r).unwrap().targets().count(), 0);
    v["destinations"] =
        json!({"strava":{},"training":{"kind":"strava"},"backup":{"kind":"strava"}});
    assert_eq!(load(&v, &r).unwrap().targets().count(), 3);
    v["destinations"] = json!({"strava":{},"bike_log":{"kind":"bryton_activity"},"connect_log":{"kind":"garmin_connect"}});
    r.register_target("bryton_activity", |id, v| {
        if v["options"]
            .as_object()
            .is_some_and(|opts| !opts.is_empty())
        {
            return Err(invalid(
                &format!("/destinations/{id}/options/typo"),
                "unexpected option",
            ));
        }
        Ok(())
    });
    assert_eq!(
        load(&v, &r).err().unwrap().details["path"],
        "/destinations/connect_log/kind"
    );
    r.register_target("garmin_connect", |_, _| Ok(()));
    assert_eq!(
        load(&v, &r)
            .unwrap()
            .targets()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["bike_log", "connect_log", "strava"]
    );
    v["destinations"] = json!({"bike_log":{"kind":"bryton_activity"}});
    assert_eq!(
        load(&v, &r)
            .unwrap()
            .targets()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>(),
        ["bike_log"]
    );
    v["destinations"]["bike_log"]["options"] = json!({"typo":true});
    assert_eq!(
        load(&v, &r).err().unwrap().details["path"],
        "/destinations/bike_log/options/typo"
    );
}

#[test]
fn invalid_config_is_diagnosed_offline() {
    let mut v = example();
    v["version"] = json!(2);
    assert_eq!(issue(&v), "/version");
    v = example();
    v["merge"]["streams"]["record.position_lat"] = json!(["bryton", "garmin"]);
    assert!(issue(&v).contains("position"));
    v = example();
    v["merge"]["streams"]["record.heart_rate"][1] = json!("apple_watch");
    assert_eq!(issue(&v), "/merge/streams/record.heart_rate/1");
    v = example();
    v["timezone"] = json!("../etc");
    assert_eq!(issue(&v), "/timezone");
    v = example();
    v["output_dir"] = v["sources"]["bryton"]["directory"].clone();
    assert_eq!(issue(&v), "/output_dir");
    assert!(
        Settings::parse(
            &EXAMPLE.replace("\"version\": 1", "\"version\": 2, \"version\": 1"),
            Path::new("."),
            &Registry::default()
        )
        .is_err()
    );
}

#[test]
fn a_config_loads_named_instances_and_priorities_offline() {
    use std::collections::BTreeSet;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut v = example();
    let r = Registry::default();
    let ids = |s: &Settings| {
        s.sources()
            .map(|s| s.id().to_owned())
            .collect::<BTreeSet<_>>()
    };
    let ordered = |s: &Settings| {
        s.selected_fields()
            .map(|f| {
                (
                    f.to_owned(),
                    s.sources_for(f)
                        .map(|s| s.id().to_owned())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<BTreeMap<_, _>>()
    };
    let target_ids = |s: &Settings| {
        s.targets()
            .map(|t| (t.id.clone(), t.kind.clone()))
            .collect::<Vec<_>>()
    };
    v["sources"]["bryton"]["directory"] = json!(dir.path().join("unmounted"));
    v["sources"]["garmin"]["cache_dir"] = json!(dir.path().join("untouched"));
    std::fs::write(&path, v.to_string()).unwrap();
    let defaults = Settings::load(&path).unwrap();
    assert_eq!(
        ids(&defaults),
        ["bryton", "garmin", "apple_watch"].map(String::from).into(),
        "A1"
    );
    assert!(
        !dir.path().join("untouched").exists(),
        "load performed source I/O"
    );
    assert_eq!(
        target_ids(&defaults),
        [("strava".into(), "strava".into())],
        "A4b"
    );
    assert!(
        defaults.selected_fields().any(|f| f == "record.heart_rate"),
        "A2a"
    );
    assert!(
        defaults
            .selected_fields()
            .any(|f| f == "session.total_calories"),
        "A2b"
    );
    v["sources"]["garmin"] =
        json!({"kind":"fit_directory","device":"garmin_cirqa","directory":"elsewhere"});
    let local = load(&v, &r).unwrap();
    assert_eq!(ids(&local), ids(&defaults), "A1d");
    assert_eq!(ordered(&local), ordered(&defaults), "A1d");
    let mut registry = Registry::default();
    registry.register_device(
        "bryton_neo15",
        Device {
            manufacturer: 267,
            product: 100,
        },
    );
    registry.register_source("test_archive", |id, _, _, _| {
        Ok(Box::new(DeferredSource {
            id: id.into(),
            kind: SourceKind::Extension("test_archive"),
        }))
    });
    for (id, spec, row) in [
        (
            "old_bike",
            json!({"kind":"fit_directory","device":"bryton_neo15","directory":"neo"}),
            "A1b",
        ),
        (
            "archive2",
            json!({"kind":"test_archive","options":{}}),
            "A1e",
        ),
    ] {
        let mut expanded = v.clone();
        expanded["sources"][id] = spec;
        let s = load(&expanded, &registry).unwrap();
        let mut expected = ids(&defaults);
        expected.insert(id.into());
        assert_eq!(ids(&s), expected, "{row}");
        assert_eq!(s.primary_source().id(), "bryton", "{row}");
        assert_eq!(ordered(&s), ordered(&defaults), "{row}");
    }
    v["destinations"] = json!({"strava":{}});
    assert_eq!(
        target_ids(&load(&v, &r).unwrap()),
        [("strava".into(), "strava".into())],
        "A4e"
    );
    v["destinations"] = json!({"training":{"kind":"strava"},"backup":{"kind":"strava"}});
    assert_eq!(
        target_ids(&load(&v, &r).unwrap()),
        [
            ("backup".into(), "strava".into()),
            ("training".into(), "strava".into())
        ],
        "A4f"
    );
}

#[test]
fn b1_b2_b3_calendar_query_and_dual_threshold() {
    use crate::data_source::ActivityQuery;
    let mut v = example();
    let unix = |s: &str| s.parse::<jiff::Timestamp>().unwrap().as_second();
    let start = unix("2026-10-13T16:00:00Z");
    let end = unix("2026-10-14T16:00:00Z");
    for ratio in [0.90, 0.95] {
        v["merge"]["min_overlap_ratio"] = json!(ratio);
        let s = load(&v, &Registry::default()).unwrap();
        assert_eq!(
            s.primary_query("2026-10-14").unwrap(),
            ActivityQuery::StartsWithin {
                start_utc: start,
                end_utc: end
            },
            "B1a/B1b"
        );
        let q = s.candidate_query(end - 600, end + 1800);
        assert_eq!(
            q,
            ActivityQuery::Overlapping {
                start_utc: end - 600,
                end_utc: end + 1800,
                minimum_overlap: ratio
            },
            "B3a/B3b"
        );
        let meta = crate::data_source::ActivityMetadata {
            source_id: "g".into(),
            activity_id: "1".into(),
            device: Device::CIRQA,
            sport: 2,
            start_utc: end - 720,
            end_utc: end + 1800,
        };
        assert!(q.matches(&meta), "B2");
        let primary = s.primary_query("2026-10-14").unwrap();
        assert!(
            primary.matches(&crate::data_source::ActivityMetadata {
                start_utc: start,
                ..meta.clone()
            }),
            "B1a"
        );
        assert!(
            !primary.matches(&crate::data_source::ActivityMetadata {
                start_utc: end,
                ..meta
            }),
            "B1b"
        );
    }
    v["timezone"] = json!("America/New_York");
    let s = load(&v, &Registry::default()).unwrap();
    let ActivityQuery::StartsWithin { start_utc, end_utc } = s.primary_query("2026-03-08").unwrap()
    else {
        panic!()
    };
    assert_eq!(end_utc - start_utc, 23 * 3600);
    assert!(s.primary_query("2026-02-30").is_err());
}
