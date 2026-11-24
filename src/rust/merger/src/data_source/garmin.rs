//! Garmin Connect Personal API. Only the official HTTPS host receives credentials.
use super::{
    ActivityMetadata, ActivityQuery, ActivityRecording, ActivitySummary, DataSource, Device,
    SourceKind,
};
use crate::{Error, Result, hash};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Cursor, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

const HOST: &str = "https://personal.connectapi.garmin.com";
const TOKEN: &str = "/personalapitoken-service/personal-api/personal-api-key/token";
const LIST: &str = "/activitylist-service/activities/search/activities";
const LIMIT: usize = 20;
const MAX_BYTES: u64 = 128 << 20;

fn protocol() -> Error {
    Error::new(
        "garmin_response",
        "unexpected Garmin Personal API response",
        6,
    )
}
fn auth_error() -> Error {
    Error::new(
        "garmin_auth",
        "update the Garmin personal API key in config",
        4,
    )
}
fn limited(input: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    input.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(Error::data("Garmin file exceeds size limit"));
    }
    Ok(bytes)
}
trait Transport {
    fn request(&self, method: &str, path: &str, authorization: &str) -> Result<Vec<u8>>;
}
struct Https(ureq::Agent);
impl Transport for Https {
    fn request(&self, method: &str, path: &str, authorization: &str) -> Result<Vec<u8>> {
        #[cfg(test)]
        super::PLATFORM_REQUESTS.with(|n| n.set(n.get() + 1));
        let response = self
            .0
            .request(method, &format!("{HOST}{path}"))
            .set("Authorization", authorization)
            .set("Accept", "*/*")
            .call();
        http_body(response)
    }
}
fn http_body(response: std::result::Result<ureq::Response, ureq::Error>) -> Result<Vec<u8>> {
    match response {
        Ok(r) if (200..300).contains(&r.status()) => limited(r.into_reader()),
        Err(ureq::Error::Status(401 | 403, _)) => Err(auth_error()),
        Err(ureq::Error::Status(429, r)) => {
            let mut e = Error::new("garmin_rate_limited", "Garmin rate limit reached", 5);
            e.details = json!({"retry_after":r.header("Retry-After")});
            Err(e)
        }
        Err(ureq::Error::Status(404, _)) => Err(Error::new(
            "activity_not_found",
            "Garmin activity unavailable",
            3,
        )),
        _ => Err(Error::new("garmin_unavailable", "Garmin request failed", 6)),
    }
}
fn lock(path: &Path) -> Result<fs::File> {
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.lock()?;
    Ok(file)
}
fn atomic(path: &Path, data: &[u8]) -> Result<()> {
    let mut tmp = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))?;
    tmp.write_all(data)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
pub(crate) struct GarminConnect {
    id: String,
    device: Device,
    cache: PathBuf,
    config: Option<PathBuf>,
    auth: RefCell<Value>,
    http: Box<dyn Transport>,
}
impl GarminConnect {
    pub fn new(
        id: &str,
        device: Device,
        cache: PathBuf,
        config: Option<&Path>,
        auth: &Value,
    ) -> Self {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout(Duration::from_secs(45))
            .build();
        Self {
            id: id.into(),
            device,
            cache,
            config: config.map(Path::to_owned),
            auth: RefCell::new(auth.clone()),
            http: Box::new(Https(agent)),
        }
    }
    fn token(&self, rejected: Option<&str>) -> Result<String> {
        let now = jiff::Timestamp::now().as_second();
        let valid = |auth: &Value| {
            let token = auth["access_token"].as_str().filter(|s| !s.is_empty())?;
            (Some(token) != rejected && auth["expires_at"].as_i64().is_none_or(|e| e > now + 30))
                .then(|| token.to_owned())
        };
        if let Some(t) = valid(&self.auth.borrow()) {
            return Ok(t);
        }
        // Re-read under a stable sibling lock so concurrent rotations preserve every config section.
        let _lock = self
            .config
            .as_ref()
            .map(|p| lock(&p.with_added_extension("lock")))
            .transpose()?;
        let mut config: Option<Value> = self
            .config
            .as_ref()
            .map(|p| -> Result<_> {
                serde_json::from_slice(&fs::read(p)?)
                    .map_err(|_| Error::usage("config changed during Garmin authentication"))
            })
            .transpose()?;
        let mut auth = if let Some(v) = &config {
            let source = &v["sources"][&self.id];
            if source["kind"] != "garmin_connect" {
                return Err(auth_error());
            }
            source["auth"].clone()
        } else {
            self.auth.borrow().clone()
        };
        if let Some(t) = valid(&auth) {
            self.auth.replace(auth);
            return Ok(t);
        }
        let key = auth["personal_api_key"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(auth_error)?;
        let bytes = self.http.request("POST", TOKEN, &format!("ApiKey {key}"))?;
        let token = std::str::from_utf8(&bytes).map_err(|_| protocol())?.trim();
        let claims = token
            .split('.')
            .nth(1)
            .and_then(|s| URL_SAFE_NO_PAD.decode(s).ok())
            .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
            .ok_or_else(protocol)?;
        let exp = claims["exp"]
            .as_i64()
            .filter(|e| *e > now)
            .ok_or_else(auth_error)?;
        auth["access_token"] = json!(token);
        auth["expires_at"] = json!(exp);
        if let (Some(config), Some(path)) = (&mut config, &self.config) {
            config["sources"][&self.id]["auth"] = auth.clone();
            atomic(
                path,
                &serde_json::to_vec_pretty(config).map_err(std::io::Error::other)?,
            )?;
        }
        self.auth.replace(auth);
        Ok(token.into())
    }
    fn get(&self, path: &str) -> Result<Vec<u8>> {
        let token = self.token(None)?;
        match self.http.request("GET", path, &format!("Bearer {token}")) {
            Err(e) if e.code == "garmin_auth" => {
                let fresh = self.token(Some(&token))?;
                self.http.request("GET", path, &format!("Bearer {fresh}"))
            }
            other => other,
        }
    }
    fn recording(&self, id: &str) -> Result<ActivityRecording> {
        let missing = || {
            Error::new(
                "activity_not_found",
                format!("source {} has no activity {id}", self.id),
                3,
            )
        };
        if id
            .parse::<u64>()
            .ok()
            .filter(|n| *n > 0)
            .map(|n| n.to_string())
            .as_deref()
            != Some(id)
        {
            return Err(missing());
        }
        let dir = self.cache.join(id);
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&dir)?;
        let _lock = lock(&dir.join("cache.lock"))?;
        let index = dir.join("sha256");
        let read = |bytes, path: &Path| -> Result<_> {
            let mut r = ActivityRecording::from_fit_bytes(&self.id, path, bytes)?;
            if r.metadata.device != self.device {
                return Err(Error::new(
                    "device_mismatch",
                    "candidate FIT belongs to another device",
                    3,
                ));
            }
            r.metadata.activity_id = id.into();
            Ok(r)
        };
        if let Ok(digest) = fs::read_to_string(&index)
            && digest.len() == 64
            && digest.bytes().all(|c| c.is_ascii_hexdigit())
        {
            let file = dir.join(format!("{digest}.fit"));
            if let Ok(bytes) = fs::read(&file)
                && hash(&bytes) == digest
            {
                return read(bytes, &file);
            }
        }
        let archive = self
            .get(&format!("/download-service/files/activity/{id}"))
            .map_err(|e| {
                if e.code == "activity_not_found" {
                    missing()
                } else {
                    e
                }
            })?;
        let mut zip = zip::ZipArchive::new(Cursor::new(archive))
            .map_err(|_| Error::data("invalid Garmin ZIP"))?;
        let entries: Vec<_> = zip
            .file_names()
            .filter(|s| s.to_ascii_lowercase().ends_with(".fit"))
            .map(str::to_owned)
            .collect();
        let [name] = entries.as_slice() else {
            return Err(Error::data("expected one FIT in the Garmin archive"));
        };
        let bytes = limited(
            zip.by_name(name)
                .map_err(|_| Error::data("unreadable FIT in ZIP"))?,
        )?;
        let digest = hash(&bytes);
        let path = dir.join(format!("{digest}.fit"));
        let recording = read(bytes, &path)?;
        atomic(&path, recording.original_bytes())?;
        atomic(&index, digest.as_bytes())?;
        Ok(recording)
    }
}
impl DataSource for GarminConnect {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> SourceKind {
        SourceKind::GarminConnect
    }
    fn list_activities(&self, query: &ActivityQuery) -> Result<Vec<ActivitySummary>> {
        // The server filters by local start date. Pad UTC dates for all civil time zones.
        let window = match *query {
            ActivityQuery::All => None,
            ActivityQuery::StartsWithin { start_utc, end_utc } => Some((start_utc, end_utc)),
            ActivityQuery::Overlapping {
                start_utc,
                end_utc,
                minimum_overlap,
            } => {
                let lead =
                    ((end_utc - start_utc) as f64 * (1.0 / minimum_overlap - 1.0)).ceil() as i64;
                Some((start_utc.saturating_sub(lead), end_utc))
            }
        };
        let mut filters = String::new();
        if let Some((start, end)) = window {
            let day = |s| {
                jiff::Timestamp::from_second(s)
                    .map(|t| t.to_zoned_datetime(jiff::tz::TimeZone::UTC).date())
                    .map_err(|_| Error::usage("Garmin date window out of range"))
            };
            let first = day(start)?.yesterday().map_err(|_| protocol())?;
            let last = day(end)?.tomorrow().map_err(|_| protocol())?;
            filters = format!("&startDate={first}&endDate={last}");
        }
        let mut result = BTreeMap::new();
        let mut pages = BTreeSet::new();
        let mut offset = 0;
        loop {
            let page = self.get(&format!(
                "{LIST}?activityType=cycling&start={offset}&limit={LIMIT}{filters}"
            ))?;
            let rows: Vec<Value> = serde_json::from_slice(&page).map_err(|_| protocol())?;
            let mut page_ids = Vec::new();
            for row in &rows {
                let id = row["activityId"]
                    .as_u64()
                    .or_else(|| row["activityId"].as_str()?.parse().ok())
                    .filter(|i| *i > 0)
                    .ok_or_else(protocol)?
                    .to_string();
                page_ids.push(id.clone());
                let start = row["beginTimestamp"]
                    .as_i64()
                    .map(|ms| ms / 1000)
                    .or_else(|| {
                        row["startTimeGMT"]
                            .as_str()?
                            .parse::<jiff::civil::DateTime>()
                            .ok()?
                            .in_tz("UTC")
                            .ok()
                            .map(|z| z.timestamp().as_second())
                    })
                    .ok_or_else(protocol)?;
                let elapsed = row["elapsedDuration"]
                    .as_f64()
                    .or_else(|| row["duration"].as_f64())
                    .filter(|n| *n > 0.0)
                    .ok_or_else(protocol)?;
                let summary = ActivitySummary {
                    source_id: self.id.clone(),
                    activity_id: id.clone(),
                    device: None,
                    start_utc: start,
                    end_utc: start
                        .checked_add(elapsed.ceil() as i64)
                        .ok_or_else(protocol)?,
                };
                if query.matches_interval(summary.start_utc, summary.end_utc) {
                    result.insert(id, summary);
                }
            }
            if rows.len() < LIMIT {
                break;
            }
            if !pages.insert(page_ids) {
                return Err(protocol());
            }
            offset += rows.len();
        }
        let mut rows: Vec<_> = result.into_values().collect();
        rows.sort_by(|a, b| (a.start_utc, &a.activity_id).cmp(&(b.start_utc, &b.activity_id)));
        Ok(rows)
    }
    fn metadata(&self, id: &str) -> Result<ActivityMetadata> {
        Ok(self.recording(id)?.metadata)
    }
    fn open_activity(&self, id: &str) -> Result<ActivityRecording> {
        self.recording(id)
    }
}
#[cfg(test)]
mod tests;
