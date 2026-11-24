use super::*;
use std::{fs, os::unix::fs::symlink};
const RIDER: &[u8] = include_bytes!("../../tests/fixtures/rider.fit");
const LATER: &[u8] = include_bytes!("../../tests/fixtures/rider-later.fit");
const CIRQA: &[u8] = include_bytes!("../../tests/fixtures/cirqa.fit");
const START: i64 = 631065600 + 0x40000000;

#[test]
fn directory_identifies_recorders_by_content_under_one_role() {
    let dir = tempfile::tempdir().unwrap();
    let source = FitDirectory::new("bike_meter", dir.path(), None);
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/identity-fixtures");
    for (file, name, origin) in [
        ("file-name.fit", "Recorder A", "file_id.product_name"),
        ("creator-name.fit", "Recorder B", "device_info.product_name"),
        ("sensor-only.fit", "FIT 267/30001", "fit_ids"),
    ] {
        let bytes = fs::read(fixtures.join(file)).unwrap();
        fs::write(dir.path().join(file), &bytes).unwrap();
        let r = source.open_activity(&crate::hash(&bytes)).unwrap();
        let device = r.metadata().recorder();
        assert_eq!((device.name.as_str(), device.name_source), (name, origin));
    }
    fs::write(dir.path().join("older-name.fit"), RIDER).unwrap();
    let rows = source.list_activities(&ActivityQuery::All).unwrap();
    assert_eq!(rows.len(), 4);
    for row in rows {
        assert_eq!(row.source_id, "bike_meter");
        let device = row.device.unwrap();
        assert_eq!(
            device,
            source.metadata(&row.activity_id).unwrap().recorder()
        );
        if row.activity_id == crate::hash(RIDER) {
            assert_eq!(
                (device.name.as_str(), device.name_source),
                ("Bryton Rider 460", "model_lookup")
            );
        }
    }
}

#[test]
fn discovers_unique_cycling_files_by_content_in_time_order() {
    let dir = tempfile::tempdir().unwrap();
    let source = FitDirectory::new("bike", dir.path(), Device::RIDER_460);
    let s: &dyn DataSource = &source;
    assert!(s.list_activities(&ActivityQuery::All).unwrap().is_empty());
    let sub = dir.path().join("nested");
    fs::create_dir(&sub).unwrap();
    fs::write(sub.join("b.FIT"), RIDER).unwrap();
    fs::write(dir.path().join("copy.fit"), RIDER).unwrap();
    fs::write(dir.path().join("1.fit"), LATER).unwrap();
    fs::write(dir.path().join("wrong.fit"), CIRQA).unwrap();
    fs::write(dir.path().join("other.zip"), b"unrelated").unwrap();
    symlink(dir.path(), sub.join("cycle")).unwrap();
    let all = s.list_activities(&ActivityQuery::All).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[1].activity_id, crate::hash(LATER), "C1c");
    assert_eq!(all[0].activity_id, crate::hash(RIDER));
    assert_eq!(all[0].source_id, "bike");
    let metadata = s.metadata(&all[0].activity_id).unwrap();
    assert_eq!(metadata.device, Device::RIDER_460);
    assert_eq!(
        (all[0].start_utc, all[0].end_utc, metadata.sport),
        (START, START + 6, 2)
    );
    assert_eq!(metadata.summary(), all[0]);
    let in_range = s
        .list_activities(&ActivityQuery::StartsWithin {
            start_utc: START,
            end_utc: START + 16,
        })
        .unwrap();
    assert_eq!(in_range, [all[0].clone()]);
    assert!(
        s.list_activities(&ActivityQuery::Overlapping {
            start_utc: START + 6,
            end_utc: START + 16,
            minimum_overlap: 0.9
        })
        .unwrap()
        .is_empty()
    );
    assert_eq!(
        s.list_activities(&ActivityQuery::Overlapping {
            start_utc: START + 5,
            end_utc: START + 7,
            minimum_overlap: 0.9
        })
        .unwrap(),
        in_range
    );
}
#[test]
fn distinguishes_missing_stream_invalid_sample_and_zero_without_reopening() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("r.fit");
    fs::write(&p, RIDER).unwrap();
    fs::write(dir.path().join("g.fit"), CIRQA).unwrap();
    let g = FitDirectory::new("watch", dir.path(), Device::CIRQA);
    let b = FitDirectory::new("bike", dir.path(), Device::RIDER_460);
    let r = b.open_activity(&crate::hash(RIDER)).unwrap();
    fs::remove_file(p).unwrap();
    assert_eq!(r.original_bytes(), RIDER);
    let samples = r.read_stream(StreamKey::Cadence).unwrap();
    assert_eq!(
        samples.iter().map(|p| p.raw).collect::<Vec<_>>(),
        [Some(0), Some(0), None, Some(0), Some(0), Some(0)]
    );
    assert_eq!(samples[2].timestamp_utc, START + 2);
    assert_eq!(
        r.read_summary(SummaryKey::SessionTotalCalories)
            .unwrap()
            .iter()
            .map(|s| s.raw)
            .collect::<Vec<_>>(),
        [Some(6)]
    );
    let watch = g.open_activity(&crate::hash(CIRQA)).unwrap();
    assert!(watch.read_stream(StreamKey::Cadence).is_none());
    assert_eq!(
        watch
            .read_summary(SummaryKey::SessionTotalCalories)
            .unwrap()
            .iter()
            .map(|s| s.raw)
            .collect::<Vec<_>>(),
        [Some(2)]
    );
}
#[test]
fn recovers_renames_and_never_opens_replacement_as_requested_activity() {
    let dir = tempfile::tempdir().unwrap();
    let old = dir.path().join("r.fit");
    let new = dir.path().join("renamed.fit");
    fs::write(&old, RIDER).unwrap();
    let s = FitDirectory::new("bike", dir.path(), Device::RIDER_460);
    let id = s
        .list_activities(&ActivityQuery::All)
        .unwrap()
        .pop()
        .unwrap()
        .activity_id;
    fs::rename(old, &new).unwrap();
    assert_eq!(s.open_activity(&id).unwrap().original_bytes(), RIDER);
    assert_eq!(
        s.list_activities(&ActivityQuery::All).unwrap()[0].activity_id,
        id
    );
    let renamed = s
        .list_activities(&ActivityQuery::All)
        .unwrap()
        .pop()
        .unwrap();
    let r = s.open_activity(&renamed.activity_id).unwrap();
    assert_eq!(r.metadata().source_id, "bike", "C7");
    assert_eq!(r.original_bytes(), RIDER, "C7");
    assert_eq!(
        r.read_stream(StreamKey::HeartRate).unwrap()[0].raw,
        Some(110)
    );
    assert_eq!(
        r.read_summary(SummaryKey::SessionTotalCalories).unwrap()[0].raw,
        Some(6)
    );
    fs::write(new, LATER).unwrap();
    assert_eq!(
        s.open_activity(&id).err().unwrap().code,
        "activity_not_found"
    );
    assert_eq!(s.metadata(&id).unwrap_err().code, "activity_not_found");
    assert!(s.metadata(&id).unwrap_err().message.contains(&id), "C4b");
    assert!(
        s.open_activity(&id).err().unwrap().message.contains(&id),
        "C4d"
    );
}
#[test]
fn missing_or_corrupt_source_is_not_a_successful_partial_list() {
    let dir = tempfile::tempdir().unwrap();
    let missing = FitDirectory::new("bike", dir.path().join("absent"), Device::RIDER_460);
    assert_eq!(
        missing
            .list_activities(&ActivityQuery::All)
            .unwrap_err()
            .code,
        "source_io"
    );
    fs::write(dir.path().join("ok.fit"), RIDER).unwrap();
    fs::write(dir.path().join("truncated.fit"), &RIDER[..20]).unwrap();
    let s = FitDirectory::new("bike", dir.path(), Device::RIDER_460);
    assert_eq!(
        s.list_activities(&ActivityQuery::All).unwrap_err().code,
        "invalid_data"
    );
}

#[test]
fn merge_accepts_already_open_recordings() {
    let dir = tempfile::tempdir().unwrap();
    let b = dir.path().join("b.fit");
    let g = dir.path().join("g.fit");
    fs::write(&b, RIDER).unwrap();
    fs::write(&g, CIRQA).unwrap();
    let primary = FitDirectory::new("my_bike", dir.path(), Device::RIDER_460);
    let backup = FitDirectory::new("my_band", dir.path(), Device::CIRQA);
    let first = primary.open_activity(&crate::hash(RIDER)).unwrap();
    let second = backup.open_activity(&crate::hash(CIRQA)).unwrap();
    fs::remove_file(b).unwrap();
    fs::remove_file(g).unwrap();
    let output = dir.path().join("merged");
    crate::merge_recordings(first, second, &output).unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(output.join("merge-report.json")).unwrap()).unwrap();
    assert_eq!(report["inputs"]["bike_meter"]["source_id"], "my_bike");
    assert_eq!(report["inputs"]["health_band"]["source_id"], "my_band");
    assert_eq!(report["heart_rate"]["samples"]["health_band"], 1);
}

#[test]
fn arbitrary_native_field_reader_exposes_arrays_zeros_and_missing_samples() {
    let r = ActivityRecording::from_fit_bytes(
        "backup",
        std::path::Path::new("backup.fit"),
        include_bytes!("../../tests/stream-fixtures/backup.fit").to_vec(),
    )
    .unwrap();
    let phases = r.read_native_stream(69).unwrap().unwrap();
    assert_eq!(phases[0].raw, Some(serde_json::json!([32, 64])));
    assert!(r.read_native_stream(240).unwrap().is_none());
    let bike = ActivityRecording::from_fit_bytes(
        "bike",
        std::path::Path::new("bike.fit"),
        include_bytes!("../../tests/stream-fixtures/bike.fit").to_vec(),
    )
    .unwrap();
    let watts = bike.read_native_stream(7).unwrap().unwrap();
    assert_eq!(watts[0].raw, Some(serde_json::json!(0)));
    assert!(watts[2].raw.is_none());
}
