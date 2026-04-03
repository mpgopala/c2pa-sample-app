# Tauri v2 + React — UI Framework Documentation

## Overview

Tauri v2 with React provides a native desktop shell around a web-based UI. The Rust `c2pa` backend runs as the Tauri core process; the frontend communicates with it via Tauri's `invoke()` command bridge.

## Architecture

```
src/                    ← React/TypeScript frontend (Vite)
  App.tsx               ← Top-level nav, page routing
  pages/
    SignPage.tsx         ← Sign workflow
    VerifyPage.tsx       ← Verify + manifest tree
    SettingsPage.tsx     ← Trust lists, config, HTTP
  styles.css            ← Global CSS custom properties

src-tauri/              ← Rust Tauri backend
  src/main.rs           ← App entry point
  Cargo.toml
```

## How to Run

```bash
# Install Node deps
npm install

# Web-only preview (no Rust required)
npm run dev
# → http://localhost:1420

# Full desktop app
npm run tauri dev

# Production build
npm run tauri build
```

## Key Design Decisions

- **No UI library dependency** — plain CSS with custom properties keeps the bundle small and avoids license concerns
- **Page routing via useState** — no React Router needed; three pages, simple state enum
- **Two-panel layout via flexbox** — left panel fixed at 280px, right panel fills remaining space with overflow-y scroll
- **Validation status via CSS classes** — `.status-verified`, `.status-tampered`, etc., driven by a `ValidationState` type
- **Tauri shell is minimal** — `src-tauri/src/main.rs` is a 5-line stub; all C2PA logic will be added as `#[tauri::command]` functions later

## Connecting to the Rust c2pa Backend

When integrating the real backend, replace button handlers with Tauri invoke calls:

```typescript
import { invoke } from "@tauri-apps/api/core";

// Example: sign a file
const result = await invoke<string>("sign_asset", {
  filePath: file,
  manifestJson: JSON.stringify(manifest),
  certPath: cert,
  keyPath: key,
});
```

## Platform Support

| Platform | Support |
|---|---|
| macOS | Native |
| Windows | Native |
| Linux | Native |
| iOS | Tauri v2 mobile (alpha) |
| Android | Tauri v2 mobile (alpha) |
| Web | Dev server only (not packaged) |

## License

- Tauri: MIT / Apache 2.0
- React: MIT
- Vite: MIT
