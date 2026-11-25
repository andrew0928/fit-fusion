use crate::query_filters::parse_date;
// R09 uses the same Garmin SDK FIT and paginated response scenarios as M2.
use super::*;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{
    cell::RefCell,
    collections::VecDeque,
    fs,
    io::{Cursor, Write},
};
const FIT: &[u8] = include_bytes!("../../merger/tests/fixtures/cirqa.fit");
struct Step(&'static str, String, &'static str, Result<Vec<u8>>);
struct Mock(RefCell<VecDeque<Step>>);
impl Transport for Mock {
    fn request(&self, method: &str, path: &str, header: &str) -> Result<Vec<u8>> {
        let step = self.0.borrow_mut().pop_front().expect("unexpected request");
        assert_eq!(method, step.0);
        assert!(path.contains(&step.1), "missing {} in {path}", step.1);
        assert!(header.starts_with(step.2));
        step.3
    }
}
fn token() -> Step {
    let part = URL_SAFE_NO_PAD.encode(json!({"exp":Timestamp::now().as_second()+3600}).to_string());
    Step(
        "POST",
        "/personal-api-key/token".into(),
        "ApiKey test-key",
        Ok(format!("t.{part}.test").into_bytes()),
    )
}
fn get(path: &str, result: Result<Vec<u8>>) -> Step {
    Step("GET", path.into(), "Bearer ", result)
}
fn client(root: &Path, steps: Vec<Step>) -> Client<Mock> {
    let file = root.join("personal.env");
    fs::write(
        &file,
        "# mock\nexport GARMIN_PERSONAL_ACCESS_TOKEN='test-key'\n",
    )
    .unwrap();
    Client::new(
        Mock(RefCell::new(steps.into())),
        Credentials { env_file: file },
    )
}
fn reply(value: Value) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(&value).unwrap())
}
fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn day_filters() -> Filters {
    Filters {
        date: Some(parse_date("2026-11-21").unwrap()),
        from: None,
        until: None,
        device: None,
    }
}
fn row(id: u64, start: &str) -> Value {
    json!({"activityId":id,"startTimeGMT":start,"elapsedDuration":3602.125})
}

#[test]
fn paginated_departure_date_deduplicates_and_keeps_cross_midnight_ride() {
    let dir = tempfile::tempdir().unwrap();
    let mut rows: Vec<_> = (1..=19).map(|id| row(id, "2026-11-20 02:00:00")).collect();
    rows.push(row(20, "2026-11-21 15:59:00"));
    let mut c = client(
        dir.path(),
        vec![
            token(),
            get("start=0&limit=20", reply(json!(rows))),
            get(
                "start=20&limit=20",
                reply(json!([
                    row(20, "2026-11-21 15:59:00"),
                    row(21, "2026-11-21 16:00:00")
                ])),
            ),
        ],
    );
    let found = c
        .list(
            day_filters()
                .window(parse_zone("Asia/Taipei").unwrap())
                .unwrap()
                .as_ref(),
        )
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, "20");
    assert_eq!(found[0].end_utc.to_string(), "2026-11-21T16:59:02.125Z");
    assert!(c.http.0.borrow().is_empty());
}
#[test]
fn repeated_page_and_failed_second_page_never_return_incomplete_list() {
    let entries = json!(
        (1..=20)
            .map(|id| row(id, "2026-11-21 06:00:00"))
            .collect::<Vec<_>>()
    );
    let dir = tempfile::tempdir().unwrap();
    for (reply2, code) in [
        (reply(entries.clone()), "garmin_response"),
        (
            Err(Error::new("garmin_unavailable", 6, "outage")),
            "garmin_unavailable",
        ),
    ] {
        let mut c = client(
            dir.path(),
            vec![
                token(),
                get("start=0", reply(entries.clone())),
                get("start=20", reply2),
            ],
        );
        assert_eq!(c.list(None).err().unwrap().code, code);
    }
}
#[test]
fn rejected_bearer_is_renewed_once_and_revoked_key_requires_reauthorization() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = client(
        dir.path(),
        vec![
            token(),
            get("start=0", Err(Error::auth("rejected"))),
            token(),
            get("start=0", reply(json!([]))),
        ],
    );
    assert!(c.list(None).unwrap().is_empty());
    assert!(c.http.0.borrow().is_empty());
    let mut c = client(
        dir.path(),
        vec![Step(
            "POST",
            "token".into(),
            "ApiKey ",
            Err(Error::auth("revoked")),
        )],
    );
    assert_eq!(c.list(None).err().unwrap().exit, 4);
}
#[test]
fn expired_bearer_is_renewed_before_next_page_and_rate_limit_retains_retry_after() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = client(
        dir.path(),
        vec![
            token(),
            get("start=0", reply(json!([]))),
            token(),
            get("start=0", Err(client::http_error(429, Some("12")))),
        ],
    );
    c.list(None).unwrap();
    c.expire_for_test();
    let error = c.list(None).err().unwrap();
    assert_eq!((error.exit, error.retry_after.as_deref()), (5, Some("12")));
    for status in [401, 403, 404, 503] {
        assert_eq!(
            client::http_error(status, None).exit,
            match status {
                401 | 403 => 4,
                404 => 3,
                _ => 6,
            }
        );
    }
}
#[test]
fn validated_zip_offline_reuse_corrupt_cache_and_direct_fit() {
    let dir = tempfile::tempdir().unwrap();
    let mut c = client(
        dir.path(),
        vec![
            token(),
            get(
                "/activity/11",
                Ok(archive(&[
                    ("notes.txt", b"metadata"),
                    ("nested/original.FIT", FIT),
                ])),
            ),
            get("/activity/11", Ok(FIT.to_vec())),
        ],
    );
    let first = cache::original(&mut c, dir.path(), 11, false).unwrap();
    assert!(!first.cache_hit);
    assert_eq!(first.bytes, FIT);
    let saved = cache::original(&mut c, dir.path(), 11, true).unwrap();
    assert!(saved.cache_hit);
    assert_eq!(saved.hash, first.hash);
    fs::write(&saved.path, b"broken").unwrap();
    assert_eq!(
        cache::original(&mut c, dir.path(), 11, true)
            .err()
            .unwrap()
            .code,
        "cache_miss"
    );
    assert_eq!(
        cache::original(&mut c, dir.path(), 11, false)
            .unwrap()
            .bytes,
        FIT
    );
    assert!(c.http.0.borrow().is_empty());
}
#[test]
fn ambiguous_or_corrupt_download_is_never_published() {
    let dir = tempfile::tempdir().unwrap();
    for response in [
        archive(&[("a.fit", FIT), ("b.fit", FIT)]),
        archive(&[("a.fit", b"corrupt")]),
        archive(&[("notes.txt", b"only")]),
    ] {
        let mut c = client(dir.path(), vec![token(), get("/activity/12", Ok(response))]);
        assert_eq!(
            cache::original(&mut c, dir.path(), 12, false)
                .err()
                .unwrap()
                .code,
            "invalid_data"
        );
        assert!(!dir.path().join("12/sha256").exists());
    }
}
#[test]
fn batch_reports_success_and_rate_limit_without_contacting_remaining_ids() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    let out = dir.path().join("export");
    let page = json!(
        (1..=3)
            .map(|id| row(id, "2026-11-21 05:00:00"))
            .collect::<Vec<_>>()
    );
    let mut c = client(
        dir.path(),
        vec![
            token(),
            get("start=0", reply(page)),
            get("/activity/1", Ok(FIT.to_vec())),
            get("/activity/2", Err(client::http_error(429, Some("30")))),
        ],
    );
    let (report, exit) = run(
        &mut c,
        Action::Export {
            id: None,
            filters: day_filters(),
            out: out.clone(),
            offline: false,
        },
        &cache,
        TimeZone::UTC,
    )
    .unwrap();
    assert_eq!(exit, 5);
    assert_eq!(report["status"], "partial");
    assert_eq!(report["exports"][2]["status"], "not_attempted");
    assert_eq!(fs::read(out.join("1.fit")).unwrap(), FIT);
    assert!(c.http.0.borrow().is_empty());
}

#[test]
fn names_follow_summary_and_verified_recorder_without_extra_remote_requests() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("cache");
    let mut entry = row(42, "2026-11-21 03:00:00");
    entry["activityName"] = json!("冷水坑");
    let page = json!([entry]);
    let mut c = client(
        dir.path(),
        vec![
            token(),
            get("start=0", reply(page.clone())),
            get("/activity/42", Ok(FIT.to_vec())),
            get("start=0", reply(page)),
        ],
    );
    let rows = selected(&mut c, &root, &day_filters(), TimeZone::UTC).unwrap();
    assert_eq!(rows[0].activity_name.as_deref(), Some("冷水坑"));
    assert!(rows[0].device_name.is_none());
    cache::original(&mut c, &root, 42, false).unwrap();
    let rows = selected(&mut c, &root, &day_filters(), TimeZone::UTC).unwrap();
    assert_eq!(rows[0].device_name.as_deref(), Some("Garmin CIRQA"));
    let (result, _) = run(
        &mut c,
        Action::Read {
            id: 42,
            offline: true,
        },
        &root,
        TimeZone::UTC,
    )
    .unwrap();
    assert_eq!(result["activity"]["activity_name"], "冷水坑");
    assert_eq!(result["activity"]["device_name"], "Garmin CIRQA");
    assert!(c.http.0.borrow().is_empty());
}
