---
phase: 01-signer-preferences-persistence
plan: "01"
subsystem: model
tags: [persistence, signer, json, rust]
dependency_graph:
  requires: []
  provides: [SignerPrefs, load_signer_prefs, save_signer_prefs]
  affects: [src/model/signer_prefs.rs, src/model/mod.rs]
tech_stack:
  added: []
  patterns: [file-based JSON persistence via serde_json, silent error fallback to defaults]
key_files:
  created:
    - src/model/signer_prefs.rs
  modified:
    - src/model/mod.rs
decisions:
  - "alg stored as String (not c2pa::SigningAlg) because SigningAlg lacks Serialize/Deserialize"
  - "Default impl is manual (not derived) so alg defaults to \"Es256\" rather than empty string"
metrics:
  duration: "~1 minute"
  completed: "2026-04-13T06:12:15Z"
  tasks_completed: 2
  files_changed: 2
---

# Phase 1 Plan 1: SignerPrefs Persistence Model Summary

**One-liner:** File-backed SignerPrefs struct with load/save functions mirroring the recents.rs JSON persistence pattern, defaulting alg to "Es256".

## What Was Built

- `src/model/signer_prefs.rs` — new module containing the `SignerPrefs` struct (`cert_path`, `key_path`, `alg` fields), a private `signer_prefs_path()` helper resolving `~/.c2pa-tool/signer_prefs.json`, public `load_signer_prefs()` that returns defaults on any error, and public `save_signer_prefs()` that writes pretty-printed JSON silently.
- `src/model/mod.rs` — added `pub mod signer_prefs;` declaration and updated the crate-level doc comment from two modules to three.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Create signer_prefs.rs model module | 5d9ab9a | src/model/signer_prefs.rs |
| 2 | Register signer_prefs module in mod.rs | 55fe9a5 | src/model/mod.rs |

## Verification Results

- `cargo check` passes (full workspace, 410 crates compiled)
- `pub struct SignerPrefs` present at line 9 of signer_prefs.rs
- `pub fn load_signer_prefs` present at line 47
- `pub fn save_signer_prefs` present at line 57
- `pub mod signer_prefs` present at line 13 of mod.rs

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None — no placeholder values or TODO stubs introduced. `cert_path` and `key_path` default to empty strings by design (user must supply paths); this is not a stub but correct initial state documented in the struct.

## Threat Flags

None — no new network endpoints, auth paths, or trust-boundary crossings introduced. The threat model in the plan (T-01-01, T-01-02, T-01-03) is fully addressed: malformed JSON falls back to defaults (T-01-03 mitigated), only paths are stored not key contents (T-01-02 accepted), tampered values only affect UI prefill (T-01-01 accepted).

## Self-Check: PASSED

- [x] `src/model/signer_prefs.rs` exists
- [x] `src/model/mod.rs` contains `pub mod signer_prefs;`
- [x] commit `5d9ab9a` exists in git log
- [x] commit `55fe9a5` exists in git log
