use crate::catalog::Activity;
use serde::Serialize;
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
pub struct Export<'a> {
    pub source_id: &'a Path,
    pub id: &'a str,
    pub sha256: &'a str,
    pub activity_name: Option<&'a str>,
    pub device_name: Option<&'a str>,
    pub path: PathBuf,
    pub format: &'static str,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub fn export<'a>(activity: &'a Activity, directory: &Path) -> Export<'a> {
    let path = directory.join(activity.path.file_name().expect("scanned file"));
    let copied = activity
        .original()
        .and_then(|bytes| fit_inspector::write::copy_original(&bytes, &path));
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
        activity_name: activity.activity_name.as_deref(),
        device_name: activity.device_name.as_deref(),
        path,
        format: "original",
        status,
        message,
    }
}
