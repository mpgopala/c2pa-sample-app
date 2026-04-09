use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Maximum number of entries kept in the recent-files list.
const MAX_RECENTS: usize = 10;

/// A single entry in the recently-opened files list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecentEntry {
    /// Full filesystem path to the file.
    pub path: String,
    /// Filename component only (for display in menus and cards).
    pub name: String,
    /// Unix timestamp (seconds) of when the file was last opened in this app.
    pub timestamp: u64,
}

/// Return the path to the persisted recents JSON file.
///
/// The file lives at `~/.c2pa-tool/recents.json` on all platforms.
fn recents_path() -> PathBuf {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".c2pa-tool").join("recents.json")
}

/// Load the persisted recent-files list from disk.
///
/// Returns an empty [`Vec`] if the file does not exist or cannot be parsed;
/// errors are silently swallowed so the app starts cleanly on first launch.
pub fn load_recents() -> Vec<RecentEntry> {
    let data = std::fs::read_to_string(recents_path()).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

/// Persist `entries` to disk as JSON, creating parent directories as needed.
///
/// Write errors are silently ignored — a stale or missing recents file is
/// not fatal.
fn save_recents(entries: &[RecentEntry]) {
    let path = recents_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(entries) {
        let _ = std::fs::write(&path, json);
    }
}

/// Prepend `path` to `entries`, deduplicating and capping the list at
/// 10 entries, then immediately persist it to disk.
///
/// If `path` already appears in the list it is moved to the front rather
/// than duplicated.  The timestamp is always refreshed to the current time.
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
