//! Model layer for the C2PA sample application.
//!
//! This crate provides three modules:
//! - [`manifest`] — reading, verifying, and signing C2PA manifests.
//! - [`recents`]  — persisting and managing recently opened file paths.
//! - [`signer_prefs`] — persisting signer certificate, key, and algorithm preferences.

/// C2PA manifest verification and signing operations.
pub mod manifest;
/// Recently-opened file list with persistent storage.
pub mod recents;
/// Signer preferences (cert path, key path, algorithm) with persistent storage.
pub mod signer_prefs;
