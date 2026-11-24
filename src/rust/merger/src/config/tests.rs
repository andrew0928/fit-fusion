use super::*;
use serde_json::json;

fn example() -> Value {
    serde_json::from_str(EXAMPLE).unwrap()
}
fn load(v: &Value, r: &Registry) -> Result<Settings> {
    Settings::parse(&v.to_string(), Path::new("."), r)
}
fn priority<'a>(s: &'a Settings, field: &str) -> Vec<&'a str> {
    s.sources_for(field).map(DataSource::id).collect()
}
fn issue(v: &Value) -> String {
    load(v, &Registry::default())
        .err()
        .expect("expected config error")
        .details["path"]
        .as_str()
        .unwrap()
        .into()
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
    v["merge"]["streams"]["record.heart_rate"][1] = json!("garmin");
    assert_eq!(issue(&v), "/merge/streams/record.heart_rate/1");
    v["sources"] = json!({"bryton":v["sources"]["bryton"]});
    v["merge"]["streams"] = json!({"record.cadence":["bryton"]});
    let only = load(&v, &r).unwrap();
    assert_eq!(only.sources().count(), 1);
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
                &format!("/destinations/{id}/options"),
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
        load(&v, &r).unwrap().targets().next().unwrap().id,
        "bike_log"
    );
    v["destinations"]["bike_log"]["options"] = json!({"typo":true});
    assert_eq!(
        load(&v, &r).err().unwrap().details["path"],
        "/destinations/bike_log/options"
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
