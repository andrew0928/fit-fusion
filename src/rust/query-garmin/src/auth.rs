use crate::{Error, Result};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub const VARIABLE: &str = "GARMIN_PERSONAL_ACCESS_TOKEN";
pub struct Credentials {
    pub env_file: Option<PathBuf>,
    pub keychain: Option<PathBuf>,
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
    fn supplied(&self) -> Result<Option<String>> {
        if let Some(file) = &self.env_file {
            return from_file(file).map(Some);
        }
        env::var_os(VARIABLE)
            .map(|v| {
                v.to_str()
                    .ok_or_else(|| Error::auth("invalid API key"))
                    .and_then(checked)
            })
            .transpose()
    }
    pub fn key(&self) -> Result<String> {
        match self.supplied()? {
            Some(key) => Ok(key),
            None => store::read(self.keychain.as_deref()),
        }
    }
    pub fn import(&self) -> Result<()> {
        let key = self.supplied()?.ok_or_else(|| {
            Error::auth("provide --env-file or GARMIN_PERSONAL_ACCESS_TOKEN to import")
        })?;
        store::save(self.keychain.as_deref(), &key)?;
        if store::read(self.keychain.as_deref())? != key {
            return Err(Error::auth(
                "Keychain read-back differs after import; retry",
            ));
        }
        Ok(())
    }
    pub fn status(&self) -> Result<()> {
        store::read(self.keychain.as_deref()).map(drop)
    }
}

#[cfg(target_os = "macos")]
mod store {
    use super::*;
    use security_framework::{base, os::macos::keychain::SecKeychain};
    const SERVICE: &str = "fit-query-garmin";
    const ACCOUNT: &str = "personal-api-key";
    fn keychain(path: Option<&Path>) -> base::Result<SecKeychain> {
        match path {
            Some(path) => SecKeychain::open(path),
            None => SecKeychain::default(),
        }
    }
    fn error(e: base::Error) -> Error {
        match e.code() {
            -25300 => {
                Error::auth("no Garmin key in Keychain; run auth import or supply --env-file")
            }
            -25308 | -25293 => Error::new(
                "keychain_locked",
                4,
                "unlock the keychain or allow this executable access, then retry",
            ),
            _ => Error::new(
                "keychain_unavailable",
                4,
                format!("macOS Keychain unavailable (OSStatus {})", e.code()),
            ),
        }
    }
    pub fn read(path: Option<&Path>) -> Result<String> {
        let _no_prompt = SecKeychain::disable_user_interaction().map_err(error)?;
        let (value, _) = keychain(path)
            .map_err(error)?
            .find_generic_password(SERVICE, ACCOUNT)
            .map_err(error)?;
        checked(std::str::from_utf8(&value).map_err(|_| Error::auth("invalid Keychain item"))?)
    }
    pub fn save(path: Option<&Path>, key: &str) -> Result<()> {
        let _no_prompt = SecKeychain::disable_user_interaction().map_err(error)?;
        let chain = keychain(path).map_err(error)?;
        match chain.find_generic_password(SERVICE, ACCOUNT) {
            Ok((_, mut item)) => item.set_password(key.as_bytes()),
            Err(e) if e.code() == -25300 => {
                chain.add_generic_password(SERVICE, ACCOUNT, key.as_bytes())
            }
            Err(e) => Err(e),
        }
        .map_err(error)
    }
}
#[cfg(not(target_os = "macos"))]
mod store {
    use super::*;
    pub fn read(_: Option<&Path>) -> Result<String> {
        Err(Error::auth(
            "macOS Keychain unavailable; provide --env-file",
        ))
    }
    pub fn save(_: Option<&Path>, _: &str) -> Result<()> {
        Err(Error::auth("auth import requires macOS"))
    }
}
