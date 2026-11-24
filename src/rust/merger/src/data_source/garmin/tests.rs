use super::*;
use crate::data_source::{StreamKey, SummaryKey};
use std::{collections::VecDeque, os::unix::fs::PermissionsExt, rc::Rc};
const ORIGINAL: &[u8] = include_bytes!("../../../tests/fixtures/cirqa.fit");
const SENSORS: &[u8] = include_bytes!("../../../tests/scenarios/cirqa-sensors.fit");
const START: i64 = 631065600 + 0x40000000;
struct Step {
    method: &'static str,
    path: String,
    auth: String,
    response: Result<Vec<u8>>,
}
struct Mock(RefCell<VecDeque<Step>>);
impl Transport for Rc<Mock> {
    fn request(&self, method: &str, path: &str, auth: &str) -> Result<Vec<u8>> {
        let next = self
            .0
            .borrow_mut()
            .pop_front()
            .expect("unexpected Garmin request");
        assert_eq!(method, next.method);
        assert!(
            path.starts_with(&next.path),
            "expected {}, got {path}",
            next.path
        );
        assert_eq!(auth, next.auth);
        next.response
    }
}
fn source(dir: &Path, steps: Vec<Step>) -> (GarminConnect, Rc<Mock>) {
    let mock = Rc::new(Mock(RefCell::new(steps.into())));
    let mut s = GarminConnect::new(
        "band",
        Device::CIRQA,
        dir.join("cache"),
        None,
        &json!({"access_token":"test-token"}),
    );
    s.http = Box::new(mock.clone());
    (s, mock)
}
fn get(path: &str, response: Result<Vec<u8>>) -> Step {
    Step {
        method: "GET",
        path: path.into(),
        auth: "Bearer test-token".into(),
        response,
    }
}
fn archive(bytes: &[u8]) -> Vec<u8> {
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    z.start_file("original.FIT", zip::write::SimpleFileOptions::default())
        .unwrap();
    z.write_all(bytes).unwrap();
    z.finish().unwrap().into_inner()
}
fn page(offset: usize, rows: Vec<Value>) -> Step {
    get(
        &format!("{LIST}?activityType=cycling&start={offset}&limit={LIMIT}"),
        Ok(serde_json::to_vec(&rows).unwrap()),
    )
}
fn row(id: u64, start: i64) -> Value {
    json!({"activityId":id,"beginTimestamp":start*1000,"elapsedDuration":6})
}
fn query() -> ActivityQuery {
    ActivityQuery::Overlapping {
        start_utc: START,
        end_utc: START + 6,
        minimum_overlap: 0.9,
    }
}
#[test]
fn c1_c2_connect_complete_pagination_and_identity() {
    let dir = tempfile::tempdir().unwrap();
    let first = (1..=LIMIT as u64).map(|i| row(i, START - 100)).collect();
    let steps = vec![
        page(0, vec![]),
        page(0, first),
        page(LIMIT, vec![row(21, START)]),
        page(0, vec![row(2, START), row(1, START), row(2, START)]),
    ];
    let (s, mock) = source(dir.path(), steps);
    assert!(s.list_activities(&query()).unwrap().is_empty(), "C1a");
    let result = s.list_activities(&query()).unwrap();
    assert_eq!(result.len(), 1, "C1b/C2");
    assert_eq!(
        (&result[0].source_id, &result[0].activity_id),
        (&"band".into(), &"21".into())
    );
    let result = s.list_activities(&query()).unwrap();
    assert_eq!(
        result
            .iter()
            .map(|r| r.activity_id.as_str())
            .collect::<Vec<_>>(),
        ["1", "2"],
        "C1c"
    );
    assert!(mock.0.borrow().is_empty());
}
#[test]
fn c3_errors_preserve_auth_rate_limit_and_failed_second_page() {
    let dir = tempfile::tempdir().unwrap();
    let (s, _) = source(
        dir.path(),
        vec![Step {
            method: "POST",
            path: TOKEN.into(),
            auth: "ApiKey expired-key".into(),
            response: Err(auth_error()),
        }],
    );
    s.auth
        .replace(json!({"access_token":"expired","expires_at":1,"personal_api_key":"expired-key"}));
    assert_eq!(s.list_activities(&query()).unwrap_err().exit, 4, "C3b");
    let (s, _) = source(
        dir.path(),
        vec![get(
            LIST,
            Err(Error::new("garmin_rate_limited", "rate limit", 5)),
        )],
    );
    assert_eq!(s.list_activities(&query()).unwrap_err().exit, 5, "C3c");
    let first = (1..=LIMIT as u64).map(|id| row(id, START)).collect();
    let (s, _) = source(dir.path(), vec![page(0, first), get(LIST, Err(protocol()))]);
    assert_eq!(s.list_activities(&query()).unwrap_err().exit, 6, "C3d");
}
#[test]
fn c4_c5_c6_cached_original_and_stream_reads() {
    let dir = tempfile::tempdir().unwrap();
    let download = "/download-service/files/activity/";
    let (s, mock) = source(
        dir.path(),
        vec![
            get(&format!("{download}10"), Ok(archive(ORIGINAL))),
            get(&format!("{download}11"), Ok(archive(SENSORS))),
            get(
                &format!("{download}12"),
                Err(Error::new("activity_not_found", "absent", 3)),
            ),
            get(
                &format!("{download}12"),
                Err(Error::new("activity_not_found", "absent", 3)),
            ),
        ],
    );
    let r = s.open_activity("10").unwrap();
    let decoded = super::super::recording::DECODE_COUNT.with(|c| c.get());
    assert_eq!(
        r.read_stream(StreamKey::HeartRate).unwrap()[0].raw,
        Some(109),
        "C5"
    );
    assert_eq!(
        r.read_summary(SummaryKey::SessionTotalCalories).unwrap()[0].raw,
        Some(2),
        "C5"
    );
    assert_eq!(
        super::super::recording::DECODE_COUNT.with(|c| c.get()),
        decoded,
        "C5"
    );
    assert_eq!(mock.0.borrow().len(), 3, "C5");
    assert_eq!(hash(r.original_bytes()), hash(ORIGINAL), "C4c");
    let meta = s.metadata("10").unwrap();
    assert_eq!(
        (meta.device, meta.sport, meta.start_utc, meta.end_utc),
        (Device::CIRQA, 2, START, START + 6),
        "C4a"
    );
    assert_eq!(meta.activity_id, "10");
    assert!(r.read_stream(StreamKey::Cadence).is_none(), "C6a");
    let r = s.open_activity("11").unwrap();
    let samples = r.read_stream(StreamKey::Cadence).unwrap();
    assert_eq!(
        (samples[2].timestamp_utc, samples[2].raw),
        (START + 2, None),
        "C6b"
    );
    assert_eq!(
        (samples[0].timestamp_utc, samples[0].raw),
        (START, Some(0)),
        "C6c"
    );
    for e in [
        s.metadata("12").unwrap_err(),
        s.open_activity("12").err().unwrap(),
    ] {
        assert_eq!(e.code, "activity_not_found", "C4b/C4d");
        assert!(e.message.contains("12"));
    }
    assert!(mock.0.borrow().is_empty());
}
#[test]
fn revoked_bearer_replaced_once_and_config_update_preserves_other_sections() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("personal.json");
    let exp = jiff::Timestamp::now().as_second() + 3600;
    let jwt = format!(
        "test.{}.test",
        URL_SAFE_NO_PAD.encode(json!({"exp":exp}).to_string())
    );
    let mut retry = page(0, vec![]);
    retry.auth = format!("Bearer {jwt}");
    let (mut s, mock) = source(
        dir.path(),
        vec![
            get(LIST, Err(auth_error())),
            Step {
                method: "POST",
                path: TOKEN.into(),
                auth: "ApiKey test-key".into(),
                response: Ok(jwt.as_bytes().into()),
            },
            retry,
        ],
    );
    let auth = json!({"personal_api_key":"test-key","access_token":"test-token"});
    let config = json!({"sources":{"band":{"kind":"garmin_connect","auth":auth}},"destinations":{"untouched":true}});
    fs::write(&path, config.to_string()).unwrap();
    s.config = Some(path.clone());
    s.list_activities(&query()).unwrap();
    let after: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(after["sources"]["band"]["auth"]["access_token"], jwt);
    assert_eq!(after["sources"]["band"]["auth"]["expires_at"], exp);
    assert_eq!(after["destinations"], config["destinations"]);
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(mock.0.borrow().is_empty());
}

#[test]
fn m10_date_merge_renews_token_deduplicates_ids_and_matches_dual_file() {
    use crate::config::{Registry, Settings};
    let dir = tempfile::tempdir().unwrap();
    let primary = dir.path().join("primary");
    fs::create_dir(&primary).unwrap();
    let rider = primary.join("ride.fit");
    fs::write(&rider, include_bytes!("../../../tests/fixtures/rider.fit")).unwrap();
    let mut registry = Registry::default();
    registry.register_source("garmin_connect", |id, _, base, _| {
        let exp = jiff::Timestamp::now().as_second() + 3600;
        let jwt = format!(
            "test.{}.test",
            URL_SAFE_NO_PAD.encode(json!({"exp":exp}).to_string())
        );
        let duplicate = vec![row(10, START), row(10, START)];
        let mut list = page(0, duplicate.clone());
        list.auth = format!("Bearer {jwt}");
        let mut download = get("/download-service/files/activity/10", Ok(archive(ORIGINAL)));
        download.auth = list.auth.clone();
        let mut rerun = page(0, duplicate);
        rerun.auth = list.auth.clone();
        let (mut s, _) = source(
            base,
            vec![
                Step {
                    method: "POST",
                    path: TOKEN.into(),
                    auth: "ApiKey test-key".into(),
                    response: Ok(jwt.into_bytes()),
                },
                list,
                download,
                rerun,
            ],
        );
        s.id = id.into();
        s.auth.replace(
            json!({"personal_api_key":"test-key","expires_at":1,"access_token":"expired"}),
        );
        Ok(Box::new(s))
    });
    let mut v: Value = serde_json::from_str(include_str!(
        "../../../../../../docs/config/config.example.json"
    ))
    .unwrap();
    v["sources"]["bryton"]["directory"] = json!(primary);
    v["output_dir"] = json!(dir.path().join("export"));
    v["sources"]["garmin"]["cache_dir"] = json!(dir.path().join("cache"));
    let config = dir.path().join("config.json");
    fs::write(&config, v.to_string()).unwrap();
    let s = Settings::load_with(&config, &registry).unwrap();
    let (batch, exit) = crate::workflow::date(&s, "2024-01-09").unwrap();
    assert_eq!(exit, 0);
    assert_eq!(batch["rides"].as_array().unwrap().len(), 1);
    let result = &batch["rides"][0];
    assert_eq!(result["matched_sources"]["garmin"]["activity_id"], "10");
    let fit = fs::read(result["artifacts"]["fit"].as_str().unwrap()).unwrap();
    let report = fs::read_to_string(result["artifacts"]["report"].as_str().unwrap()).unwrap();
    for secret in ["test-key", "Bearer", "expired"] {
        assert!(!report.contains(secret));
        assert!(!batch.to_string().contains(secret));
    }
    let watch = dir.path().join("watch.fit");
    fs::write(&watch, ORIGINAL).unwrap();
    let direct = crate::merge_files([rider, watch], dir.path().join("pair")).unwrap();
    assert_eq!(
        fit,
        fs::read(direct["artifacts"]["fit"].as_str().unwrap()).unwrap()
    );
    let (again, exit) = crate::workflow::date(&s, "2024-01-09").unwrap();
    assert_eq!(exit, 0);
    assert_eq!(again["rides"][0]["reused"], true);
}

#[test]
fn http_responses_distinguish_revocation_rate_limit_and_outage() {
    for (status, code, exit) in [
        (401, "garmin_auth", 4),
        (403, "garmin_auth", 4),
        (429, "garmin_rate_limited", 5),
        (500, "garmin_unavailable", 6),
    ] {
        let response: ureq::Response =
            format!("HTTP/1.1 {status} Error\r\nRetry-After: 12\r\n\r\nprivate response")
                .parse()
                .unwrap();
        let error = http_body(Err(ureq::Error::Status(status, response))).unwrap_err();
        assert_eq!((error.code, error.exit), (code, exit));
        assert!(!error.message.contains("private"));
        if status == 429 {
            assert_eq!(error.details["retry_after"], "12");
        }
    }
}
