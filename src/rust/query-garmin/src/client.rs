//! Read-only Garmin Connect Personal API. Redirects are disabled for authenticated requests.
use crate::{Error, Result, auth::Credentials};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jiff::{
    Timestamp,
    civil::{Date, DateTime},
    tz::TimeZone,
};
use serde::Serialize;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    io::{Cursor, Read},
    ops::Range,
    time::Duration,
};

const HOST: &str = "https://personal.connectapi.garmin.com";
const TOKEN: &str = "/personalapitoken-service/personal-api/personal-api-key/token";
const LIST: &str = "/activitylist-service/activities/search/activities";
const LIMIT: usize = 20;
const MAX_BYTES: u64 = 128 << 20;

fn limited(reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(MAX_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(Error::data("Garmin response exceeds size limit"));
    }
    Ok(bytes)
}

pub trait Transport {
    fn request(&self, method: &str, path: &str, auth: &str) -> Result<Vec<u8>>;
}
pub struct Https(ureq::Agent);
impl Https {
    pub fn new() -> Self {
        Self(
            ureq::AgentBuilder::new()
                .redirects(0)
                .timeout(Duration::from_secs(45))
                .build(),
        )
    }
}
impl Transport for Https {
    fn request(&self, method: &str, path: &str, auth: &str) -> Result<Vec<u8>> {
        match self
            .0
            .request(method, &format!("{HOST}{path}"))
            .set("Authorization", auth)
            .call()
        {
            Ok(r) if (200..300).contains(&r.status()) => limited(r.into_reader()),
            Err(ureq::Error::Status(status, r)) => Err(http_error(status, r.header("Retry-After"))),
            _ => Err(Error::new("garmin_unavailable", 6, "Garmin request failed")),
        }
    }
}
pub(super) fn http_error(status: u16, retry: Option<&str>) -> Error {
    match status {
        401 | 403 => {
            Error::auth("Garmin rejected the credential; renew the Personal API key and retry")
        }
        404 => Error::new("activity_not_found", 3, "Garmin activity unavailable"),
        429 => {
            let mut e = Error::new(
                "garmin_rate_limited",
                5,
                "Garmin rate limit reached; retry after the indicated delay",
            );
            e.retry_after = retry.map(str::to_owned);
            e
        }
        _ => Error::new(
            "garmin_unavailable",
            6,
            format!("Garmin returned HTTP {status}"),
        ),
    }
}
pub struct Client<T> {
    pub http: T,
    pub credentials: Credentials,
    bearer: Option<(String, i64)>,
}
#[derive(Serialize)]
pub struct Summary {
    pub source_id: &'static str,
    pub id: String,
    pub start_utc: Timestamp,
    pub end_utc: Timestamp,
    pub device: Option<Value>,
    pub activity_name: Option<String>,
    pub device_name: Option<String>,
}
pub struct Window {
    pub days: Range<Date>,
    pub zone: TimeZone,
}
impl<T: Transport> Client<T> {
    pub fn new(http: T, credentials: Credentials) -> Self {
        Self {
            http,
            credentials,
            bearer: None,
        }
    }
    #[cfg(test)]
    pub(super) fn expire_for_test(&mut self) {
        self.bearer.as_mut().expect("issued bearer").1 = 0;
    }
    fn bearer(&mut self) -> Result<&str> {
        let now = Timestamp::now().as_second();
        if self
            .bearer
            .as_ref()
            .is_none_or(|(_, expires)| *expires <= now + 30)
        {
            let key = self.credentials.key()?;
            let body = self.http.request("POST", TOKEN, &format!("ApiKey {key}"))?;
            let token = std::str::from_utf8(&body)
                .map_err(|_| Error::protocol())?
                .trim();
            let claims = token
                .split('.')
                .nth(1)
                .and_then(|s| URL_SAFE_NO_PAD.decode(s).ok())
                .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                .ok_or_else(Error::protocol)?;
            let expires = claims["exp"]
                .as_i64()
                .filter(|exp| *exp > now)
                .ok_or_else(|| {
                    Error::auth("Garmin issued an expired access token; renew the Personal API key")
                })?;
            self.bearer = Some((token.into(), expires));
        }
        Ok(&self.bearer.as_ref().unwrap().0)
    }
    fn get(&mut self, path: &str) -> Result<Vec<u8>> {
        let mut header = format!("Bearer {}", self.bearer()?);
        match self.http.request("GET", path, &header) {
            Err(e) if e.code == "garmin_auth" => {
                self.bearer = None;
                header = format!("Bearer {}", self.bearer()?);
                self.http.request("GET", path, &header)
            }
            other => other,
        }
    }
    pub fn list(&mut self, window: Option<&Window>) -> Result<Vec<Summary>> {
        let mut filters = String::new();
        if let Some(w) = window {
            // Connect filters by device-local dates; apply the user's departure zone after pagination.
            let utc_day = |d: Date| {
                d.at(0, 0, 0, 0)
                    .to_zoned_datetime(w.zone.clone())
                    .map(|z| z.timestamp().to_zoned_datetime(TimeZone::UTC).date())
                    .map_err(|_| Error::usage("invalid departure date"))
            };
            let first = utc_day(w.days.start)?
                .yesterday()
                .map_err(|_| Error::usage("start date out of range"))?;
            let last = utc_day(w.days.end)?
                .tomorrow()
                .map_err(|_| Error::usage("end date out of range"))?;
            filters = format!("&startDate={first}&endDate={last}");
        }
        let mut result = BTreeMap::new();
        let mut seen_pages = HashSet::new();
        let mut start = 0;
        loop {
            let page = self.get(&format!(
                "{LIST}?activityType=cycling&start={start}&limit={LIMIT}{filters}"
            ))?;
            let rows: Vec<Value> = serde_json::from_slice(&page).map_err(|_| Error::protocol())?;
            let mut ids = Vec::new();
            for row in &rows {
                let id = row["activityId"]
                    .as_u64()
                    .or_else(|| row["activityId"].as_str()?.parse().ok())
                    .filter(|id| *id > 0)
                    .ok_or_else(Error::protocol)?;
                ids.push(id);
                let departure = row["beginTimestamp"]
                    .as_i64()
                    .and_then(|ms| Timestamp::from_millisecond(ms).ok())
                    .or_else(|| {
                        row["startTimeGMT"]
                            .as_str()?
                            .parse::<DateTime>()
                            .ok()?
                            .in_tz("UTC")
                            .ok()
                            .map(|z| z.timestamp())
                    })
                    .ok_or_else(Error::protocol)?;
                if window.is_some_and(|w| {
                    !w.days
                        .contains(&departure.to_zoned_datetime(w.zone.clone()).date())
                }) {
                    continue;
                }
                let seconds = row["elapsedDuration"]
                    .as_f64()
                    .or_else(|| row["duration"].as_f64())
                    .filter(|s| *s >= 0.)
                    .ok_or_else(Error::protocol)?;
                let end = departure
                    .as_millisecond()
                    .checked_add((seconds * 1000.).round() as i64)
                    .and_then(|ms| Timestamp::from_millisecond(ms).ok())
                    .ok_or_else(Error::protocol)?;
                result.insert(
                    id,
                    Summary {
                        source_id: "garmin_connect",
                        id: id.to_string(),
                        device: None,
                        activity_name: row["activityName"]
                            .as_str()
                            .map(str::trim)
                            .filter(|name| !name.is_empty())
                            .map(str::to_owned),
                        device_name: row["deviceName"]
                            .as_str()
                            .map(str::trim)
                            .filter(|name| !name.is_empty())
                            .map(str::to_owned),
                        start_utc: departure,
                        end_utc: end,
                    },
                );
            }
            if rows.len() < LIMIT {
                break;
            }
            if !seen_pages.insert(ids) {
                return Err(Error::protocol());
            }
            start += rows.len();
        }
        let mut result: Vec<_> = result.into_values().collect();
        result.sort_by(|a, b| (a.start_utc, &a.id).cmp(&(b.start_utc, &b.id)));
        Ok(result)
    }
    pub fn download(&mut self, id: u64) -> Result<Vec<u8>> {
        let body = self.get(&format!("/download-service/files/activity/{id}"))?;
        if body.get(8..12) == Some(b".FIT") {
            return Ok(body);
        }
        let mut archive = zip::ZipArchive::new(Cursor::new(body))
            .map_err(|_| Error::data("expected Garmin FIT or ZIP"))?;
        let mut selected = None;
        for i in 0..archive.len() {
            let file = archive
                .by_index(i)
                .map_err(|_| Error::data("unreadable Garmin ZIP member"))?;
            if !file.is_dir()
                && file.name().to_ascii_lowercase().ends_with(".fit")
                && selected.replace(i).is_some()
            {
                return Err(Error::data("more than one FIT member in Garmin ZIP"));
            }
        }
        let file = archive
            .by_index(selected.ok_or_else(|| Error::data("no FIT member in Garmin ZIP"))?)
            .map_err(|_| Error::data("unreadable Garmin FIT member"))?;
        limited(file)
    }
}
