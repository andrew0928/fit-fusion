use crate::{Error, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub const VARIABLE: &str = "GARMIN_PERSONAL_ACCESS_TOKEN";
pub struct Credentials {
    pub env_file: PathBuf,
}

fn checked(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::auth("invalid or missing Garmin Personal API key"));
    }
    Ok(value.into())
}

fn from_file(file: &Path) -> Result<String> {
    let content = fs::read_to_string(file).map_err(|_| Error::auth("cannot read --env-file"))?;
    for line in content.lines() {
        let line = line.trim().strip_prefix("export ").unwrap_or(line.trim());
        if let Some((name, value)) = line.split_once('=')
            && name.trim() == VARIABLE
        {
            let value = value.trim();
            let unquoted = value
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(value);
            return checked(unquoted);
        }
    }
    Err(Error::auth(
        "GARMIN_PERSONAL_ACCESS_TOKEN missing from --env-file",
    ))
}

impl Credentials {
    pub fn key(&self) -> Result<String> {
        from_file(&self.env_file)
    }
}
