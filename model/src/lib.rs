//! Model layer for the C2PA sample application.
//!
//! This crate provides three modules:
//! - [`manifest`] — reading, verifying, and signing C2PA manifests.
//! - [`recents`]  — persisting and managing recently opened file paths.
//! - [`preferences`] — persisting application preferences (cert path, key path, algorithm).

/// C2PA manifest verification and signing operations.
pub mod manifest;
/// Recently-opened file list with persistent storage.
pub mod recents;
/// Application preferences with persistent storage.
pub mod preferences;
