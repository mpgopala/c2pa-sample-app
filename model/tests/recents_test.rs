//! Integration tests for `model::recents`.
//!
//! These tests override `HOME` to redirect persistence into a per-test
//! temp directory. Because `set_var` is process-global, all tests in this
//! file are serialised through a `Mutex`.

use std::path::PathBuf;
use std::sync::Mutex;

use model::recents::{load_recents, push_recent, RecentEntry};

static HOME_LOCK: Mutex<()> = Mutex::new(());

struct HomeGuard {
    prev_home: Option<String>,
    prev_userprofile: Option<String>,
    dir: PathBuf,
}

impl HomeGuard {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir()
            .join(format!("c2pa-tool-test-recents-{}-{}", std::process::id(), label));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let prev_home = std::env::var("HOME").ok();
        let prev_userprofile = std::env::var("USERPROFILE").ok();
        unsafe {
            std::env::set_var("HOME", &dir);
            std::env::set_var("USERPROFILE", &dir);
        }
        HomeGuard { prev_home, prev_userprofile, dir }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.prev_home {
                Some(v) => std::env::set_var("HOME", v),
                None => std::env::remove_var("HOME"),
            }
            match &self.prev_userprofile {
                Some(v) => std::env::set_var("USERPROFILE", v),
                None => std::env::remove_var("USERPROFILE"),
            }
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn load_returns_empty_when_file_missing() {
    let _g = HOME_LOCK.lock().unwrap();
    let _h = HomeGuard::new("load-missing");
    let entries = load_recents();
    assert!(entries.is_empty());
}

#[test]
fn push_then_load_round_trips() {
    let _g = HOME_LOCK.lock().unwrap();
    let _h = HomeGuard::new("round-trip");
    let mut entries: Vec<RecentEntry> = Vec::new();
    push_recent("/tmp/sample.jpg", &mut entries);
    let reloaded = load_recents();
    assert_eq!(reloaded.len(), 1);
    assert_eq!(reloaded[0].path, "/tmp/sample.jpg");
    assert_eq!(reloaded[0].name, "sample.jpg");
}

#[test]
fn push_dedups_existing_path_and_moves_to_front() {
    let _g = HOME_LOCK.lock().unwrap();
    let _h = HomeGuard::new("dedup");
    let mut entries: Vec<RecentEntry> = Vec::new();
    push_recent("/tmp/a.jpg", &mut entries);
    push_recent("/tmp/b.jpg", &mut entries);
    push_recent("/tmp/a.jpg", &mut entries);
    assert_eq!(entries.len(), 2, "duplicate path must not grow the list");
    assert_eq!(entries[0].path, "/tmp/a.jpg", "re-pushed entry must be at front");
    assert_eq!(entries[1].path, "/tmp/b.jpg");
}

#[test]
fn push_caps_list_at_max_recents() {
    let _g = HOME_LOCK.lock().unwrap();
    let _h = HomeGuard::new("cap");
    let mut entries: Vec<RecentEntry> = Vec::new();
    for i in 0..15 {
        push_recent(&format!("/tmp/file_{i}.jpg"), &mut entries);
    }
    assert_eq!(entries.len(), 10, "list must be capped at MAX_RECENTS = 10");
    assert_eq!(entries[0].path, "/tmp/file_14.jpg", "newest must be at front");
    assert_eq!(entries[9].path, "/tmp/file_5.jpg", "oldest retained must be #5");
}

#[test]
fn push_extracts_filename_component() {
    let _g = HOME_LOCK.lock().unwrap();
    let _h = HomeGuard::new("name");
    let mut entries: Vec<RecentEntry> = Vec::new();
    push_recent("/some/deep/path/asset.png", &mut entries);
    assert_eq!(entries[0].name, "asset.png");
}
