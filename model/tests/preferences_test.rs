//! Integration tests for `model::preferences`.
//!
//! These tests override `HOME` to redirect persistence into a per-test
//! temp directory. Because `set_var` is process-global, all tests in this
//! file are serialised through a `Mutex`.

use std::path::PathBuf;
use std::sync::Mutex;

use model::preferences::{load_preferences, save_preferences, Preferences};

static HOME_LOCK: Mutex<()> = Mutex::new(());

struct HomeGuard {
    prev_home: Option<String>,
    prev_userprofile: Option<String>,
    dir: PathBuf,
}

impl HomeGuard {
    fn new(label: &str) -> Self {
        let dir = std::env::temp_dir()
            .join(format!("c2pa-tool-test-prefs-{}-{}", std::process::id(), label));
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
fn default_has_es256_and_empty_paths() {
    let prefs = Preferences::default();
    assert_eq!(prefs.alg, "Es256");
    assert!(prefs.cert_path.is_empty());
    assert!(prefs.key_path.is_empty());
}

#[test]
fn load_returns_default_when_file_missing() {
    let _g = HOME_LOCK.lock().unwrap();
    let _h = HomeGuard::new("missing");
    let prefs = load_preferences();
    assert_eq!(prefs, Preferences::default());
}

#[test]
fn save_then_load_round_trips() {
    let _g = HOME_LOCK.lock().unwrap();
    let _h = HomeGuard::new("round-trip");
    let prefs = Preferences {
        cert_path: "/tmp/cert.pem".to_string(),
        key_path: "/tmp/key.pem".to_string(),
        alg: "Es384".to_string(),
    };
    save_preferences(&prefs);
    let reloaded = load_preferences();
    assert_eq!(reloaded, prefs);
}

#[test]
fn load_returns_default_when_file_corrupt() {
    let _g = HOME_LOCK.lock().unwrap();
    let h = HomeGuard::new("corrupt");
    let prefs_dir = h.dir.join(".c2pa-tool");
    std::fs::create_dir_all(&prefs_dir).unwrap();
    std::fs::write(prefs_dir.join("preferences.json"), b"{ this is not json").unwrap();
    let prefs = load_preferences();
    assert_eq!(prefs, Preferences::default(), "corrupt file must fall back to defaults");
}
