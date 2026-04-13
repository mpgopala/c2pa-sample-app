# C2PA Sample App

## What This Is

A desktop tool for signing and verifying C2PA manifests. Two independent UI implementations (Dioxus pure-Rust and Tauri Rust+JS) share the same Rust model layer (`src/model/`). Targets developers and content creators who need to embed or inspect C2PA provenance data in images, video, and PDFs.

## Core Value

A user can sign any supported asset with their own certificate and immediately verify the resulting manifest — end to end, on the desktop, without a server.

## Current Milestone: v1.1 UX Polish

**Goal:** Improve daily usability by persisting signer inputs across launches and hiding the log panel by default.

**Target features:**
- Persist Signer panel inputs across app launches (cert path, key path, algorithm)
- Log panel starts collapsed/hidden by default

## Requirements

### Validated

<!-- Shipped in v1.0 baseline -->

- ✓ User can sign an asset with a certificate and private key — v1.0
- ✓ User can add an unsigned manifest archive (.c2pa) — v1.0
- ✓ User can verify an asset and see validation status (TRUSTED/VALID/INVALID/NO MANIFEST) — v1.0
- ✓ User can inspect the full manifest store as a collapsible tree — v1.0
- ✓ App remembers the last 10 verified files across launches — v1.0
- ✓ Live log pane with text filter, level filter, auto-scroll, and clear — v1.0
- ✓ Both Dioxus and Tauri UIs ship with feature parity — v1.0

### Active

- [ ] Signer panel inputs persist across app launches (cert path, key path, algorithm)
- [ ] Log panel starts collapsed/hidden by default on launch

### Out of Scope

- Settings tab persistence and actual config loading — Settings UI is a prototype; wiring it up is deferred
- Remote manifest fetching — HTTP resolution toggle is UI-only, not wired
- Trust list management — UI exists, backend not connected

## Context

- Rust workspace with two UI crates: `ui/dioxus` (pure Rust, Dioxus framework) and `ui/tauri/src-tauri` (Tauri backend + vanilla JS frontend)
- Shared model layer in `src/model/` — `manifest.rs` (sign/verify) and `recents.rs` (recent files)
- Recents already persists to `~/.c2pa-tool/recents.json` — similar pattern for signer prefs
- Log pane visibility is currently toggled by a handle; initial state defaults to open

## Constraints

- **Tech stack**: Rust + Dioxus or Rust + Tauri + vanilla JS — no new frameworks
- **Persistence**: Use file-based JSON (same pattern as recents) for signer prefs — no external DB

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Two UI implementations sharing one model layer | Allows framework comparison; model stays framework-agnostic | ✓ Good |
| File-based JSON for recents persistence | Simple, portable, no deps | ✓ Good |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-04-13 after v1.1 milestone start*
