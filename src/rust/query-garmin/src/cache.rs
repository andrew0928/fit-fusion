//! Each Garmin activity ID has a validated original and an atomically published SHA-256.
use crate::{
    Error, Result,
    client::{Client, Transport},
};
use fit_inspector::activity::{self, ActivityFile};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

pub struct Original {
    pub bytes: Vec<u8>,
    pub hash: String,
    pub path: PathBuf,
    pub cache_hit: bool,
    pub metadata: ActivityFile,
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temp = tempfile::NamedTempFile::new_in(path.parent().expect("cache directory"))?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}
fn checked(bytes: Vec<u8>, hash: String, path: PathBuf, cache_hit: bool) -> Result<Original> {
    let metadata = activity::scan(&bytes).map_err(|e| Error::data(e.to_string()))?;
    if !metadata.is_activity {
        return Err(Error::data("Garmin original is not an activity FIT"));
    }
    Ok(Original {
        bytes,
        hash,
        path,
        cache_hit,
        metadata,
    })
}
fn cached(dir: &Path) -> Result<Option<Original>> {
    let hash = match fs::read_to_string(dir.join("sha256")) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if hash.len() != 64 || !hash.bytes().all(|v| v.is_ascii_hexdigit()) {
        return Ok(None);
    }
    let path = dir.join(format!("{hash}.fit"));
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if format!("{:x}", Sha256::digest(&bytes)) != hash {
        return Ok(None);
    }
    checked(bytes, hash, path, true).map(Some)
}
/// An ordinary list can use an existing original without downloading one.
pub fn cached_original(root: &Path, id: &str) -> Result<Option<Original>> {
    cached(&root.join(id))
}

pub fn activity_name(root: &Path, id: &str) -> Option<String> {
    fs::read_to_string(root.join(id).join("activity_name"))
        .ok()
        .filter(|s| !s.is_empty())
}

/// Titles live in Connect summaries. Keep the last observation for ID-only and offline reads.
pub fn remember_name(root: &Path, id: &str, name: Option<&str>) -> Result<()> {
    if activity_name(root, id).as_deref() == name {
        return Ok(());
    }
    let dir = root.join(id);
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    atomic(
        &dir.join("activity_name"),
        name.unwrap_or_default().as_bytes(),
    )
}
pub fn original<T: Transport>(
    client: &mut Client<T>,
    root: &Path,
    id: u64,
    offline: bool,
) -> Result<Original> {
    let dir = root.join(id.to_string());
    if let Some(hit) = cached(&dir)? {
        return Ok(hit);
    }
    if offline {
        return Err(Error::new(
            "cache_miss",
            3,
            format!("no verified cached original for {id}"),
        ));
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)?;
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .read(true)
        .mode(0o600)
        .open(dir.join("cache.lock"))?;
    lock.lock()?;
    if let Some(hit) = cached(&dir)? {
        return Ok(hit);
    }
    let bytes = client.download(id)?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let path = dir.join(format!("{hash}.fit"));
    let result = checked(bytes, hash, path, false)?;
    atomic(&result.path, &result.bytes)?;
    atomic(&dir.join("sha256"), result.hash.as_bytes())?;
    Ok(result)
}
