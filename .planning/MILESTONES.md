# Milestones

## v1.0 — Baseline (shipped)

**Completed:** 2026-04-13 (pre-GSD baseline)

### What shipped

- Sign asset — embed C2PA manifest into image/video/PDF with custom assertions and ingredients
- Add manifest (unsigned) — export `.c2pa` archive without signing
- Verify asset — validate manifest, show TRUSTED/VALID/INVALID/NO MANIFEST status
- Manifest store tree — collapsible inspection of full manifest hierarchy
- Ingredient link resolution — JUMBF references highlight inline
- Recent files — last 10 verified files persisted to `~/.c2pa-tool/recents.json`
- Log pane — live filterable log with level filter, auto-scroll, clear
- Two UIs: Dioxus (pure Rust) and Tauri (Rust + vanilla JS) with feature parity
- Settings tab (prototype — not wired)

### Phases

(Pre-GSD — no phase breakdown recorded)
