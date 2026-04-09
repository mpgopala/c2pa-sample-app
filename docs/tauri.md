# Tauri v2 + Vanilla JS — UI Framework Documentation

## Overview

Tauri v2 with a plain HTML/CSS/JavaScript frontend. The Rust `c2pa` backend runs as the Tauri core process; the frontend communicates with it via Tauri's `invoke()` command bridge. No npm build step, no framework — just a static `index.html` served directly from the filesystem.

## Architecture

```
ui/tauri/
  index.html              ← Shell: nav, page-content div, log pane
  styles.css              ← Same CSS as the Dioxus prototype
  app.js                  ← All frontend logic (ES module, no build step)
  package.json            ← @tauri-apps/cli devDependency only

  src-tauri/
    Cargo.toml            ← Tauri 2 crate, depends on c2pa-sample-app
    build.rs              ← tauri_build::build()
    tauri.conf.json       ← Window config; withGlobalTauri: true
    capabilities/
      default.json        ← Permissions: core:default, dialog:allow-open/save
    src/
      main.rs             ← Entry point: c2pa_tauri_lib::run()
      lib.rs              ← Plugin registration + invoke_handler
      commands.rs         ← Tauri commands wrapping the model layer
      logger.rs           ← tracing Layer; LogEntry serialised to JS
```

## How to Run

```bash
# One-time: install Tauri CLI
cd ui/tauri
npm install

# Development (auto-reloads on file save with Tauri CLI)
cd ui/tauri/src-tauri
cargo tauri dev

# Or just compile and run without the CLI
cargo run
```

## Tauri Commands

All communication between JS and Rust goes through `invoke()`.

| Command | Args | Returns |
|---------|------|---------|
| `verify_asset` | `{ path }` | `VerifyResult` |
| `sign_asset_cmd` | `{ params: SignParamsDto }` | `Result<String>` |
| `add_manifest_cmd` | `{ params: ManifestParamsDto, dest }` | `Result<String>` |
| `load_recents_cmd` | — | `Vec<RecentEntry>` |
| `push_recent_cmd` | `{ path }` | `Vec<RecentEntry>` |
| `drain_logs_cmd` | — | `Vec<LogEntry>` |

Called from JS:
```js
const invoke = window.__TAURI__.core.invoke;

// Verify
const result = await invoke('verify_asset', { path: '/path/to/image.jpg' });

// Sign
const dest = await invoke('sign_asset_cmd', {
  params: {
    manifest: { source, title, format: null, assertions, ingredients },
    dest, cert_path, key_path, alg: 'Es256',
  },
});
```

## File Dialogs

File picking uses `tauri-plugin-dialog`, exposed as `window.__TAURI__.dialog.open()` and `.save()` when `withGlobalTauri: true` is set in `tauri.conf.json`.

```js
const path = await window.__TAURI__.dialog.open({
  filters: [{ name: 'Images', extensions: ['jpg', 'png'] }],
});
```

## Key Design Decisions

- **No build step** — `frontendDist: "../"` in `tauri.conf.json` points Tauri directly at the directory containing `index.html`. No Vite, no Webpack.
- **`withGlobalTauri: true`** — injects `window.__TAURI__` into the WebView, making `invoke` and plugin APIs available without ES module imports or a bundler.
- **Global `state` object** — a single mutable JS object holds all page state. `renderPage()` re-renders the current page's HTML on every state change.
- **Log polling** — `setInterval(pollLogs, 200)` calls `drain_logs_cmd` every 200 ms and appends new entries to the log pane, mirroring the Dioxus `use_coroutine` approach.
- **Same CSS** — `styles.css` is identical to `ui/dioxus/src/styles.css`, so the visual design is consistent between both UIs.
- **No native menu bar** — the Tauri menu API (`tauri-plugin-menu`) can be added later; for now, navigation is handled entirely in the HTML nav bar.

## Comparison with Dioxus

| | Dioxus | Tauri (this) |
|-|--------|--------------|
| Language | Rust + RSX macros | Rust backend, JS frontend |
| Reactivity | Dioxus signals | Manual `renderPage()` on state mutation |
| File dialogs | `rfd::AsyncFileDialog` | `tauri-plugin-dialog` |
| Native menu | `muda` crate | Not yet (can add `tauri-plugin-menu`) |
| CSS delivery | `include_str!` macro (embedded in binary) | External file loaded by WebView |
| Build step | `cargo build` only | `cargo build` only (no npm build) |

## Platform Support

| Platform | Support |
|----------|---------|
| macOS | Native |
| Windows | Native |
| Linux | Native (WebKitGTK required) |
| iOS | Tauri v2 mobile (experimental) |
| Android | Tauri v2 mobile (experimental) |

## License

- Tauri: MIT / Apache 2.0
