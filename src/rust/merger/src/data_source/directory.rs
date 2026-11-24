use super::{
    ActivityMetadata, ActivityQuery, ActivityRecording, ActivitySummary, DataSource, Device,
    SourceKind, recording,
};
use crate::{Error, Result};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone)]
struct Entry {
    path: PathBuf,
    metadata: ActivityMetadata,
}

pub struct FitDirectory {
    id: String,
    directory: PathBuf,
    device: Option<Device>,
    index: RefCell<BTreeMap<String, Entry>>,
}
impl FitDirectory {
    /// Construction only describes the source. A missing mount is reported on first use.
    pub fn new(
        id: impl Into<String>,
        directory: impl Into<PathBuf>,
        device: impl Into<Option<Device>>,
    ) -> Self {
        Self {
            id: id.into(),
            directory: directory.into(),
            device: device.into(),
            index: RefCell::default(),
        }
    }
    /// Manual dual-file entry: bypass directory enumeration when the caller supplies the file.
    pub(crate) fn open_file(path: &Path) -> Result<ActivityRecording> {
        ActivityRecording::read(path, "").map_err(|e| {
            if e.code == "source_io" {
                Error::data(e.message)
            } else {
                e
            }
        })
    }
    fn refresh(&self) -> Result<()> {
        #[cfg(test)]
        super::SOURCE_READS.with(|n| n.set(n.get() + 1));
        let mut index = BTreeMap::new();
        let mut dirs = vec![self.directory.clone()];
        while let Some(dir) = dirs.pop() {
            for item in fs::read_dir(&dir).map_err(|e| io(&dir, e))? {
                let item = item.map_err(|e| io(&dir, e))?;
                let path = item.path();
                let ty = item.file_type().map_err(|e| io(&path, e))?;
                if ty.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if !ty.is_file()
                    || !path
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("fit"))
                {
                    continue;
                }
                let bytes = fs::read(&path).map_err(|e| io(&path, e))?;
                if let Some(meta) =
                    recording::inventory(&bytes, &self.id, &path, self.device)?.metadata
                {
                    index
                        .entry(meta.activity_id.clone())
                        .and_modify(|e: &mut Entry| {
                            if path < e.path {
                                e.path = path.clone();
                            }
                        })
                        .or_insert(Entry {
                            path,
                            metadata: meta,
                        });
                }
            }
        }
        self.index.replace(index);
        Ok(())
    }
    fn missing(&self, id: &str) -> Error {
        Error::new(
            "activity_not_found",
            format!("source {} has no activity {id}", self.id),
            3,
        )
    }
}
fn io(path: &Path, e: std::io::Error) -> Error {
    Error::new("source_io", format!("{}: {e}", path.display()), 3)
}
impl DataSource for FitDirectory {
    fn id(&self) -> &str {
        &self.id
    }
    fn kind(&self) -> SourceKind {
        SourceKind::FitDirectory
    }
    fn list_activities(&self, query: &ActivityQuery) -> Result<Vec<ActivitySummary>> {
        self.refresh()?;
        let mut rows: Vec<_> = self
            .index
            .borrow()
            .values()
            .map(|e| &e.metadata)
            .filter(|m| query.matches(m))
            .map(ActivityMetadata::summary)
            .collect();
        rows.sort_by(|a, b| (a.start_utc, &a.activity_id).cmp(&(b.start_utc, &b.activity_id)));
        Ok(rows)
    }
    fn metadata(&self, id: &str) -> Result<ActivityMetadata> {
        if let Some(entry) = self.index.borrow().get(id)
            && let Ok(bytes) = fs::read(&entry.path)
            && crate::hash(&bytes) == id
        {
            return Ok(entry.metadata.clone());
        }
        self.refresh()?;
        self.index
            .borrow()
            .get(id)
            .map(|e| e.metadata.clone())
            .ok_or_else(|| self.missing(id))
    }
    fn open_activity(&self, id: &str) -> Result<ActivityRecording> {
        // An old index may point at a renamed or replaced file. Re-index once before giving up.
        if let Some(entry) = self.index.borrow().get(id)
            && let Ok(recording) = ActivityRecording::read(&entry.path, &self.id)
            && recording.metadata().activity_id == id
        {
            return Ok(recording);
        }
        self.refresh()?;
        let index = self.index.borrow();
        let entry = index.get(id).ok_or_else(|| self.missing(id))?;
        let recording = ActivityRecording::read(&entry.path, &self.id)?;
        if recording.metadata().activity_id != id {
            return Err(self.missing(id));
        }
        Ok(recording)
    }
}
