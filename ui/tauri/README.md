# C2PA Tool — Tauri Prototype

## Prerequisites

- Rust stable toolchain
- Node.js (for `@tauri-apps/cli`)

## Run

```bash
# Install Tauri CLI (one-time)
npm install

# Run
cd src-tauri
cargo tauri dev
```

Or without the Tauri CLI:

```bash
cd src-tauri
cargo run
```

## Build

```bash
cd src-tauri
cargo tauri build
```

## Architecture

- **Backend**: `src-tauri/src/` — Rust commands wrapping the `c2pa-sample-app` model crate
- **Frontend**: `index.html` + `styles.css` + `app.js` — vanilla JS, no build step

See [../../docs/tauri.md](../../docs/tauri.md) for full documentation.
