# Dioxus — UI Framework Documentation

## Overview

Dioxus is a Rust-native UI framework with JSX-like syntax. The entire app — UI and logic — is written in Rust. No JavaScript, no FFI. The `c2pa` crate can be imported directly alongside Dioxus components.

## Architecture

```
src/
  main.rs             ← Entry point: dioxus::launch(App)
  app.rs              ← Root component, nav, page routing
  styles.css          ← Embedded CSS (include_str!)
  pages/
    mod.rs
    sign.rs           ← Sign page component
    verify.rs         ← Verify page component
    settings.rs       ← Settings page component
```

## How to Run

```bash
# Standard run
cargo run

# With hot reload (requires dioxus-cli)
cargo install dioxus-cli
dx serve --platform desktop
```

## Key Design Decisions

- **CSS embedded via `include_str!`** — the stylesheet is compiled into the binary, no external file deployment needed
- **`use_signal` for all state** — Dioxus 0.6's reactive primitive; works like React's `useState` but Rust-typed
- **Page routing via signal enum** — `Signal<Page>` holds `Page::Sign | Page::Verify | Page::Settings`; `match` in the template renders the active page
- **No macros for event handlers** — closures capture signals by `move`, mutations via `.set()` and `.write()`
- **Styles shared with Tauri React** — identical CSS class names mean the visual design is consistent across both prototypes for easy comparison

## Integrating the c2pa Crate

Since everything is Rust, integration is a direct import:

```rust
// In Cargo.toml: c2pa = "0.78.6"
use c2pa::{Builder, Reader};

async fn sign_file(path: &str) -> Result<(), c2pa::Error> {
    let builder = Builder::from_json(manifest_json)?;
    builder.save_to_file(path)?;
    Ok(())
}
```

Call from a component with `spawn()` for async:

```rust
button {
    onclick: move |_| {
        spawn(async move {
            sign_file(&path).await.unwrap();
        });
    },
    "Sign Asset"
}
```

## Platform Support

| Platform | Support |
|---|---|
| macOS | Native |
| Windows | Native |
| Linux | Native |
| iOS | Experimental (Dioxus 0.6) |
| Android | Experimental (Dioxus 0.6) |
| Web (WASM) | Supported |

## License

- Dioxus: MIT
