//! Append-only JSONL checkpoint store: one record per accepted unit.
//! Crash-safe by construction (append + flush); resuming replays the file.

use crate::LlmError;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointEntry {
    pub id: String,
    pub translation: String,
}

#[derive(Debug)]
pub struct CheckpointStore {
    path: PathBuf,
}

impl CheckpointStore {
    pub fn open(path: &Path) -> Result<Self, LlmError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LlmError::CheckpointIo(format!("{}: {e}", path.display())))?;
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Completed unit ids -> translations from any previous run.
    pub fn load_completed(&self) -> Result<std::collections::BTreeMap<String, String>, LlmError> {
        let mut out = std::collections::BTreeMap::new();
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return Ok(out);
        };
        for line in raw.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: CheckpointEntry = serde_json::from_str(line)
                .map_err(|e| LlmError::CheckpointIo(format!("corrupt checkpoint line: {e}")))?;
            out.insert(entry.id, entry.translation);
        }
        Ok(out)
    }

    pub fn record(&self, entry: &CheckpointEntry) -> Result<(), LlmError> {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| LlmError::CheckpointIo(format!("{}: {e}", self.path.display())))?;
        let mut line =
            serde_json::to_string(entry).map_err(|e| LlmError::CheckpointIo(e.to_string()))?;
        line.push('\n');
        f.write_all(line.as_bytes())
            .and_then(|_| f.flush())
            .map_err(|e| LlmError::CheckpointIo(format!("{}: {e}", self.path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_replays_records() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("cp.jsonl");
        let store = CheckpointStore::open(&p).unwrap();
        store
            .record(&CheckpointEntry {
                id: "a".into(),
                translation: "А".into(),
            })
            .unwrap();
        let reopened = CheckpointStore::open(&p).unwrap();
        let done = reopened.load_completed().unwrap();
        assert_eq!(done.get("a").map(String::as_str), Some("А"));
    }
}
