use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_RECENTS: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentEntry {
    /// Full filesystem path to the file.
    pub path: String,
    /// Filename component only (for display).
    pub name: String,
    /// Unix timestamp (seconds) of when the file was last opened.
    pub timestamp: u64,
}

fn recents_path() -> PathBuf {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".c2pa-tool").join("recents.json")
}

/// Load persisted recent entries, returning an empty list on any error.
pub fn load_recents() -> Vec<RecentEntry> {
    let data = std::fs::read_to_string(recents_path()).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_recents(entries: &[RecentEntry]) {
    let path = recents_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(&path, json);
    }
}

/// Add `path` to the front of `entries`, deduplicating and capping at
/// `MAX_RECENTS`.  Persists immediately.
pub fn push_recent(path: &str, entries: &mut Vec<RecentEntry>) {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let name = PathBuf::from(path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    entries.retain(|e| e.path != path);
    entries.insert(0, RecentEntry { path: path.to_string(), name, timestamp });
    entries.truncate(MAX_RECENTS);
    save_recents(entries);
}
