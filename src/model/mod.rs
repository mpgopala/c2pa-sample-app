//! Model layer for the C2PA sample application.
//!
//! This crate provides two modules:
//! - [`manifest`] — reading, verifying, and signing C2PA manifests.
//! - [`recents`]  — persisting and managing recently opened file paths.

/// C2PA manifest verification and signing operations.
pub mod manifest;
/// Recently-opened file list with persistent storage.
pub mod recents;
