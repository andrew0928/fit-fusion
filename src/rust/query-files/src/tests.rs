use super::*;
use crate::catalog::{digest, list};
use crate::query_filters::parse_date;
use fit_inspector::activity::Device;
use std::{fs, os::unix::fs::symlink, thread};

const RIDER: &[u8] = include_bytes!("../../merger/tests/fixtures/rider.fit");
const CIRQA: &[u8] = include_bytes!("../../merger/tests/fixtures/cirqa.fit");
const LATER: &[u8] = include_bytes!("../../merger/tests/fixtures/rider-later.fit");

fn all() -> Query {
    Filters::default().query(TimeZone::UTC, None).unwrap()
}

#[test]
fn departure_date_uses_the_requested_zone_and_exclusive_end() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("ride.fit"), RIDER).unwrap();
    for (from, until, zone, count) in [
        ("2024-01-09", "2024-01-10", "Asia/Taipei", 1),
        ("2024-01-08", "2024-01-09", "Asia/Taipei", 0),
        ("2024-01-09", "2024-01-10", "Pacific/Kiritimati", 0),
        ("2024-01-10", "2024-01-11", "Pacific/Kiritimati", 1),
    ] {
        let filters = Filters {
            from: Some(parse_date(from).unwrap()),
            until: Some(parse_date(until).unwrap()),
            ..Default::default()
        };
        let query = filters.query(parse_zone(zone).unwrap(), None).unwrap();
        assert_eq!(
            list(dir.path(), &query, None).unwrap().activities.len(),
            count
        );
    }
}

#[test]
fn mixed_devices_and_renamed_copies_have_stable_ids() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let nested = root.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(root.join("a.FIT"), RIDER).unwrap();
    fs::write(nested.join("other-name.fit"), RIDER).unwrap();
    fs::write(root.join("garmin.fit"), CIRQA).unwrap();
    fs::write(nested.join("later.fit"), LATER).unwrap();
    symlink(root, nested.join("loop")).unwrap();
    let q = Query {
        device: Some(Device::RIDER_460),
        ..all()
    };
    let first = list(root, &q, None).unwrap();
    assert!(first.errors.is_empty());
    assert_eq!(
        first
            .activities
            .iter()
            .map(|a| a.id.clone())
            .collect::<Vec<_>>(),
        [digest(RIDER), digest(LATER)]
    );
    assert!(first.activities[0].path.ends_with("a.FIT"));
    fs::rename(root.join("a.FIT"), root.join("z.fit")).unwrap();
    let q = Query {
        id: Some(digest(RIDER)),
        ..all()
    };
    let result = list(root, &q, None).unwrap();
    assert_eq!(result.activities.len(), 1);
    assert!(result.activities[0].path.ends_with("nested/other-name.fit"));
    assert_eq!(
        result.activities[0].device.as_ref().unwrap().name,
        "Bryton Rider 460"
    );
}

#[test]
fn bad_source_is_reported_and_replacement_cannot_be_exported_under_old_id() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("ride.fit");
    fs::write(&file, RIDER).unwrap();
    fs::write(dir.path().join("broken.fit"), b"bad crc").unwrap();
    let rows = list(dir.path(), &all(), None).unwrap();
    assert_eq!((rows.activities.len(), rows.errors.len()), (1, 1));
    let ride = &rows.activities[0];
    assert_eq!(ride.original().unwrap(), RIDER);
    fs::write(file, CIRQA).unwrap();
    let dest = dir.path().join("export");
    assert_eq!(export::export(ride, &dest).status, "error");
    assert!(!dest.exists());
}

#[test]
fn record_streams_use_valid_sample_times_and_include_valid_zero() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("ride.fit"), RIDER).unwrap();
    let activity = list(dir.path(), &all(), None)
        .unwrap()
        .activities
        .pop()
        .unwrap();
    let streams = activity.streams().unwrap();
    let cadence = streams
        .iter()
        .find(|s| s["name"] == "record.cadence")
        .unwrap();
    assert_eq!(cadence["sample_count"], 5);
    assert_eq!(cadence["start_utc"], "2024-01-09T13:37:04Z");
    assert_eq!(cadence["end_utc"], "2024-01-09T13:37:09Z");
    assert!(!streams.iter().any(|s| s["name"] == "record.timestamp"));
}

#[test]
fn concurrent_exports_publish_one_original_and_protect_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let dest = tempfile::tempdir().unwrap();
    let source = dir.path().join("ride.fit");
    fs::write(&source, RIDER).unwrap();
    let activity = list(dir.path(), &all(), None)
        .unwrap()
        .activities
        .pop()
        .unwrap();
    let results = thread::scope(|scope| {
        let jobs = (0..4)
            .map(|_| scope.spawn(|| export::export(&activity, dest.path()).status))
            .collect::<Vec<_>>();
        jobs.into_iter()
            .map(|job| job.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|s| **s == "exported").count(), 1);
    assert_eq!(results.iter().filter(|s| **s == "reused").count(), 3);
    let target = dest.path().join("ride.fit");
    assert_eq!(fs::read(&target).unwrap(), RIDER);
    fs::remove_file(&target).unwrap();
    symlink(&source, &target).unwrap();
    assert_eq!(export::export(&activity, dest.path()).status, "conflict");
    assert_eq!(fs::read(&source).unwrap(), RIDER);
}
