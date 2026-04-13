use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Signer preferences persisted across app launches.
///
/// Stores the certificate path, private key path, and signing algorithm
/// chosen by the user so they do not need to re-enter them on every launch.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignerPrefs {
    /// Filesystem path to the signing certificate (PEM).
    pub cert_path: String,
    /// Filesystem path to the private key file.
    pub key_path: String,
    /// Signing algorithm name, matching the Debug format of `c2pa::SigningAlg`
    /// (e.g. `"Es256"`, `"Es384"`, `"Es512"`, `"Ps256"`, `"Ps384"`, `"Ps512"`, `"Ed25519"`).
    /// Stored as a plain String because `c2pa::SigningAlg` does not implement
    /// `Serialize`/`Deserialize`.
    pub alg: String,
}

impl Default for SignerPrefs {
    fn default() -> Self {
        SignerPrefs {
            cert_path: String::new(),
            key_path: String::new(),
            alg: "Es256".to_string(),
        }
    }
}

/// Return the path to the persisted signer preferences JSON file.
///
/// The file lives at `~/.c2pa-tool/signer_prefs.json` on all platforms.
fn signer_prefs_path() -> PathBuf {
    let base = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    base.join(".c2pa-tool").join("signer_prefs.json")
}

/// Load the persisted signer preferences from disk.
///
/// Returns [`SignerPrefs::default()`] if the file does not exist or cannot be
/// parsed; errors are silently swallowed so the app starts cleanly on first
/// launch.
pub fn load_signer_prefs() -> SignerPrefs {
    let data = std::fs::read_to_string(signer_prefs_path()).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

/// Persist `prefs` to disk as pretty-printed JSON, creating parent
/// directories as needed.
///
/// Write errors are silently ignored — a stale or missing prefs file is not
/// fatal; the app will simply fall back to defaults on next launch.
pub fn save_signer_prefs(prefs: &SignerPrefs) {
    let path = signer_prefs_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(prefs) {
        let _ = std::fs::write(&path, json);
    }
}
