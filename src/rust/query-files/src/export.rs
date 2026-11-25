use crate::catalog::Activity;
use serde::Serialize;
use std::{
    fs,
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct Export<'a> {
    pub source_id: &'a Path,
    pub id: &'a str,
    pub sha256: &'a str,
    pub path: PathBuf,
    pub format: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub fn export<'a>(activity: &'a Activity, directory: &Path) -> Export<'a> {
    let path = directory.join(activity.path.file_name().expect("scanned file"));
    let copied = copy_original(activity, directory, &path);
    let (status, message) = match copied {
        Ok(status) => (status, None),
        Err(error) => (
            if error.kind() == ErrorKind::AlreadyExists {
                "conflict"
            } else {
                "error"
            },
            Some(error.to_string()),
        ),
    };
    Export {
        source_id: &activity.source_id,
        id: &activity.id,
        sha256: &activity.id,
        path,
        format: "original",
        status,
        message,
    }
}

fn copy_original(activity: &Activity, directory: &Path, dest: &Path) -> io::Result<&'static str> {
    let bytes = activity.original()?;
    fs::create_dir_all(directory)?;
    let existing = || -> io::Result<&'static str> {
        if dest.symlink_metadata()?.is_file() && fs::read(dest)? == bytes {
            Ok("reused")
        } else {
            Err(io::Error::new(
                ErrorKind::AlreadyExists,
                "destination already exists with different contents or file type",
            ))
        }
    };
    match existing() {
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        result => return result,
    }
    // Publish a complete sibling file exactly once, including competing exporters.
    let mut staged = tempfile::NamedTempFile::new_in(directory)?;
    staged.write_all(&bytes)?;
    staged.as_file().sync_all()?;
    match staged.persist_noclobber(dest) {
        Ok(_) => Ok("exported"),
        Err(e) if e.error.kind() == ErrorKind::AlreadyExists => existing(),
        Err(e) => Err(e.error),
    }
}
