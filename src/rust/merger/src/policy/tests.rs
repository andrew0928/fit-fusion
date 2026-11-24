use super::*;
use crate::data_source::{ActivityRecording, number, scalar};
use rustyfit::proto::Value as FitValue;
use std::path::Path;

const RIDER: &[u8] = include_bytes!("../../tests/scenarios/rider-gps.fit");
const CIRQA: &[u8] = include_bytes!("../../tests/fixtures/cirqa.fit");
const START: u32 = 0x40000000;
fn fixture(n: usize) -> MatchedRide {
    let mut rides = BTreeMap::new();
    for (id, bytes) in [("bryton", RIDER), ("garmin", CIRQA)] {
        let r = ActivityRecording::from_fit_bytes(id, Path::new("fixture"), bytes.into()).unwrap();
        let mut ride = Ride::from_recording(r).unwrap();
        ride.samples = (0..n)
            .map(|i| (START + i as u32, Some(130 + (i % 2) as u8)))
            .collect();
        rides.insert(id.into(), ride);
    }
    MatchedRide {
        primary_id: "bryton".into(),
        rides,
    }
}
fn hr(pair: &mut MatchedRide, source: &str, i: usize, v: Option<u8>) {
    pair.rides
        .get_mut(source)
        .unwrap()
        .samples
        .insert(START + i as u32, v);
}
fn select(pair: &MatchedRide) -> Result<SelectedRide<'_>> {
    Settings::embedded().unwrap().merge_policy().select(pair)
}
fn missing(pair: &mut MatchedRide, count: usize, step: usize) {
    for i in 0..count {
        hr(pair, "bryton", i * step, None);
    }
}
#[test]
fn h1_h2_h3_preferred_fill_and_unfilled_seconds() {
    let mut p = fixture(100);
    hr(&mut p, "garmin", 0, Some(135));
    hr(&mut p, "garmin", 1, Some(135));
    hr(&mut p, "bryton", 1, None);
    hr(&mut p, "bryton", 3, None);
    hr(&mut p, "garmin", 3, None);
    let points = select(&p).unwrap().heart.points;
    for (i, expected, source, row) in [
        (0, Some(130), "bryton", "H1"),
        (1, Some(135), "garmin", "H2"),
        (2, Some(130), "bryton", "H2"),
        (3, None, "missing", "H3"),
    ] {
        assert_eq!(
            (points[i].hr, points[i].time, points[i].source),
            (expected, START + i as u32, source),
            "{row}"
        );
    }
}
#[test]
fn h4_h5_h6_confirmed_and_unconfirmed_plateaus() {
    let mut p = fixture(200);
    for i in 90..180 {
        hr(&mut p, "bryton", i, Some(120));
        hr(&mut p, "garmin", i, Some(120 + (i % 2) as u8));
    }
    let s = select(&p).unwrap().heart;
    assert_eq!(s.report["mode"], "bryton_with_gap_fill", "H4");
    assert_eq!(s.report["plateaus_without_confirmation"][0]["seconds"], 90);
    for i in 90..180 {
        hr(&mut p, "garmin", i, Some(130 + (i % 10) as u8));
    }
    hr(&mut p, "garmin", 10, None);
    let s = select(&p).unwrap().heart;
    assert_eq!(s.report["triggers"][0]["reason"], "stale", "H5");
    assert_eq!(s.points[0].source, "garmin", "H5");
    assert_eq!(
        (s.points[10].hr, s.points[10].source),
        (None, "missing"),
        "H6"
    );
}
#[test]
fn h7a_h7b_missing_or_unhealthy_backup() {
    let mut p = fixture(1000);
    missing(&mut p, 600, 1);
    for i in 0..600 {
        hr(&mut p, "garmin", i, Some(123));
    }
    let error = select(&p).err().unwrap();
    assert_eq!(error.code, "quality_review", "H7b");
    assert_eq!(error.details["garmin_longest_constant_seconds"], 600);
    p.rides.remove("garmin");
    let error = select(&p).err().unwrap();
    assert_eq!(error.code, "quality_review", "H7a");
    assert_eq!(error.details["reason"], "missing_backup");
}
#[test]
fn q1_q2_missing_duration_and_fraction_boundaries() {
    for (row, n, gaps, step, whole, reason) in [
        ("Q1a", 12100, 599, 1, false, ""),
        ("Q1b", 12100, 600, 1, true, "long_missing"),
        ("Q1c", 60, 60, 1, true, "missing_coverage"),
        ("Q2a", 2380, 119, 10, false, ""),
        ("Q2b", 2401, 120, 10, false, ""),
        ("Q2c", 2400, 120, 10, true, "missing_coverage"),
    ] {
        let mut p = fixture(n);
        missing(&mut p, gaps, step);
        let s = select(&p).unwrap().heart;
        assert_eq!(s.only_garmin, whole, "{row}");
        assert_eq!(
            s.report["samples"]["garmin"],
            if whole { n } else { gaps },
            "{row}"
        );
        if whole {
            assert_eq!(s.report["triggers"][0]["reason"], reason, "{row}");
        }
    }
}
#[test]
fn q3_q4_plateau_evidence_and_pause_boundaries() {
    for (row, n, valid, spread, run, delta, pause, whole) in [
        ("Q3a", 59, 59, 5, 20, 5, false, false),
        ("Q3b", 60, 57, 5, 20, 5, false, true),
        ("Q3c", 60, 56, 5, 20, 5, false, false),
        ("Q3d", 60, 57, 4, 20, 5, false, false),
        ("Q3e", 60, 57, 5, 19, 5, false, false),
        ("Q3f", 60, 57, 5, 20, 4, false, false),
        ("Q4", 60, 60, 5, 20, 5, true, false),
    ] {
        let mut p = fixture(120);
        for i in 0..n {
            hr(&mut p, "bryton", 20 + i, Some(120));
            let v = if i < run {
                120 + delta
            } else if delta == 4 && i % 2 == 0 {
                119
            } else {
                120 + delta - spread.min(delta)
            };
            hr(&mut p, "garmin", 20 + i, (i < valid).then_some(v));
        }
        if pause {
            p.rides
                .get_mut("bryton")
                .unwrap()
                .boundaries
                .insert(START + 50);
        }
        let s = select(&p).unwrap().heart;
        assert_eq!(s.only_garmin, whole, "{row}");
        if whole {
            assert_eq!(s.report["triggers"][0]["reason"], "stale", "{row}");
        } else if n == 60 && !pause {
            assert_eq!(
                s.report["plateaus_without_confirmation"][0]["seconds"], 60,
                "{row}"
            );
        } else {
            assert!(
                s.report["plateaus_without_confirmation"]
                    .as_array()
                    .unwrap()
                    .is_empty(),
                "{row}"
            );
        }
    }
}
#[test]
fn q5_whole_and_affected_coverage_are_independent() {
    for (row, lost_inside, lost_total, ok) in [
        ("Q5a", 36, 50, false),
        ("Q5b", 30, 60, false),
        ("Q5c", 30, 50, true),
    ] {
        let mut p = fixture(1000);
        missing(&mut p, 600, 1);
        for i in 0..lost_inside {
            hr(&mut p, "garmin", i * 10, None);
        }
        for i in 0..lost_total - lost_inside {
            hr(&mut p, "garmin", 600 + i * 10, None);
        }
        let result = select(&p);
        if ok {
            assert!(result.unwrap().heart.only_garmin, "{row}");
        } else {
            let e = result.err().unwrap();
            assert_eq!(e.code, "quality_review", "{row}");
            assert_eq!(e.details["garmin_valid"], 1000 - lost_total, "{row}");
            assert_eq!(
                e.details["garmin_valid_affected"],
                600 - lost_inside,
                "{row}"
            );
        }
    }
}
#[test]
fn f1_f2_f3_session_calories_missing_and_invalid_difference() {
    let mut p = fixture(6);
    let set = |ride: &mut Ride, id, value| {
        ride.session
            .fields
            .iter_mut()
            .find(|f| f.num == id)
            .unwrap()
            .value = FitValue::Uint16(value)
    };
    set(p.rides.get_mut("bryton").unwrap(), 11, 6186);
    for (row, total, metabolic, active) in [
        ("F1", 1897, 383, Some(1514)),
        ("F3a", 100, 65535, None),
        ("F3b", 100, 101, None),
    ] {
        set(p.rides.get_mut("garmin").unwrap(), 11, total);
        set(p.rides.get_mut("garmin").unwrap(), 196, metabolic);
        let s = select(&p).unwrap();
        let c = s.calories.unwrap();
        assert_eq!(c.total, Some(total), "{row}");
        assert_eq!(
            c.metabolic,
            (metabolic != 65535).then_some(metabolic),
            "{row}"
        );
        assert_eq!(c.active(), active, "{row}");
        assert_eq!(number(&p.primary().session, 11), Some(6186), "F1");
    }
    set(p.rides.get_mut("garmin").unwrap(), 11, 65535);
    let e = select(&p).err().unwrap();
    assert_eq!(e.code, "quality_review", "F2");
    assert!(e.message.contains("total_calories"), "F2");
}
#[test]
fn f4_f5_omit_temperature_and_select_coordinates_in_pairs() {
    let p = fixture(6);
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../../../docs/config/config.example.json"
    ))
    .unwrap();
    v["merge"]["streams"]
        .as_object_mut()
        .unwrap()
        .remove("record.temperature");
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), v.to_string()).unwrap();
    let settings = Settings::load(tmp.path()).unwrap();
    let s = settings.merge_policy().select(&p).unwrap();
    assert!(s.fields.iter().all(|f| f.id != 13), "F4");
    assert_eq!(scalar(&p.primary().records[&START], 13), Some(22));
    for id in [0, 1] {
        let f = s.fields.iter().find(|f| f.id == id).unwrap();
        assert_eq!(f.values[2], None, "F5");
        assert_eq!(f.values.len(), 6);
    }
    let fit = crate::output::build(p.primary(), &s).unwrap();
    for m in crate::data_source::decode(&fit).unwrap().messages {
        if m.num.0 == 18 {
            assert!(!m.fields.iter().any(|f| f.num == 57), "F4 summary");
        }
    }
}
