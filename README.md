# C2PA Sample App

A desktop tool for signing and verifying [C2PA](https://c2pa.org/) manifests. Two independent UI implementations share the same Rust model layer.

## What it does

| Feature | Description |
|---------|-------------|
| **Sign Asset** | Embed a C2PA manifest into an image, video, or PDF. Add assertions (actions, training/mining rights, CreativeWork metadata), attach ingredients, and sign with your own certificate and private key. |
| **Add Manifest (unsigned)** | Export a `.c2pa` manifest archive without signing — useful for inspection or tooling. |
| **Verify Asset** | Read and validate the C2PA manifest store embedded in a file. Inspect the full manifest tree, validation status, thumbnail, and ingredient chain. |
| **Recent Files** | The last 10 verified files are remembered across launches (`~/.c2pa-tool/recents.json`). |
| **Log Pane** | A live, filterable log pane at the bottom shows all operations in real time. |

---

## Project layout

```
c2pa-sample-app/
  src/
    model/
      manifest.rs     — verify_embedded_manifest(), sign_asset(), add_manifest()
      recents.rs      — load_recents(), push_recent()
  ui/
    dioxus/           — Desktop app: pure Rust + Dioxus (no JS)
    tauri/            — Desktop app: Tauri backend + vanilla JS/HTML/CSS frontend
  docs/               — Architecture and framework documentation
```

---

## Quick start

### Dioxus UI (pure Rust)

```bash
cd ui/dioxus
cargo run
```

With hot reload:
```bash
cargo install dioxus-cli
dx serve --platform desktop
```

### Tauri UI (Rust + vanilla JS)

```bash
# One-time: install Tauri CLI
npm install          # from ui/tauri/

# Run
cd ui/tauri/src-tauri
cargo tauri dev
# or without Tauri CLI:
cargo run
```

---

## Using the app

### Signing an asset

1. Open the **Sign** tab.
2. Click **Browse** under "Drop file here" and select your source asset (JPEG, PNG, MP4, MOV, PDF, TIFF, WebP).
3. The **Signed Output File** and **Manifest Archive** paths are derived automatically — change them if needed.
4. Under **Signer**, pick an algorithm (ES256 is the default) and browse to your certificate and private key (both PEM format).
5. Under **Assertions**, add one or more assertions from the preset list or type a custom label:
   - `c2pa.actions` — add one or more action entries (created, edited, published, …) with optional digital source type
   - `c2pa.training-mining` — set AI training/mining permissions
   - `stds.schema-org.CreativeWork` — embed author name and copyright notice
   - Any other label uses a raw JSON editor
6. Optionally add **Ingredients** (other files referenced by this asset).
7. Click **Sign Asset** to embed the manifest in a new signed copy, or **Add Manifest** to export an unsigned `.c2pa` archive.

**Generating a test certificate/key:**
```bash
openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 \
    -keyout key.pem -out cert.pem -days 365 -nodes \
    -subj "/CN=Test Signer"
```

### Verifying an asset

1. Open the **Verify** tab.
2. Click **Browse** and select any file.
3. The left panel shows:
   - The validation badge: **TRUSTED**, **VALID**, **INVALID**, or **NO MANIFEST**
   - The signing certificate issuer and timestamp
   - An embedded thumbnail (if present)
4. The right panel shows the full **Manifest Store** as a collapsible tree. Click any section header to expand or collapse it.
5. Ingredient entries with a JUMBF reference show an **↗ ingredient** link — clicking it expands and highlights the referenced ingredient row.
6. **Validation Results** below the tree lists raw C2PA validation status codes.

Recent files appear in the left panel for quick re-opening.

### Settings

The **Settings** tab controls:
- **Trust Lists** — PEM files containing trusted CA certificates used during validation.
- **Configuration** — load a TOML config file or paste JSON inline.
- **HTTP Resolution** — toggle remote manifest fetching and set a timeout.

> Note: Settings UI is currently a prototype. Persistence and actual config loading are not yet wired up.

### Log pane

The log pane at the bottom of the window captures all `tracing` events:
- **Filter** by text (searches both message and target).
- **Filter by level** — All / Trace+ / Debug+ / Info+ / Warn+ / Error only.
- **Auto-scroll** — enabled by default; uncheck to pause scrolling.
- **Clear** — empties the log buffer.
- Drag the handle between the page content and the log pane to resize it.

---

## Model layer API

The `c2pa_sample_app` crate (`src/`) exposes these public functions:

```rust
use c2pa_sample_app::model::manifest::{
    verify_embedded_manifest,   // -> VerifyResult
    sign_asset,                 // (SignParams) -> Result<String, String>
    add_manifest,               // (ManifestParams, dest) -> Result<String, String>
    SigningAlg,
};
use c2pa_sample_app::model::recents::{
    load_recents,   // -> Vec<RecentEntry>
    push_recent,    // (&str, &mut Vec<RecentEntry>)
};
```

**Verify a file:**
```rust
let result = verify_embedded_manifest("/path/to/image.jpg");
match result.state {
    VerifyValidationState::Trusted    => println!("Trusted"),
    VerifyValidationState::Valid      => println!("Valid"),
    VerifyValidationState::Invalid    => println!("Invalid / tampered"),
    VerifyValidationState::NoManifest => println!("No C2PA data"),
}
if let Some(m) = result.manifest {
    println!("Signed by: {:?}", m.issuer);
    println!("{} assertions", m.assertions.len());
}
```

**Sign a file:**
```rust
use serde_json::json;

let result = sign_asset(SignParams {
    manifest: ManifestParams {
        source: "/path/to/photo.jpg".into(),
        title: Some("My Photo".into()),
        format: None,  // inferred from extension
        assertions: vec![
            ("c2pa.actions".into(), json!({ "actions": [{ "action": "c2pa.created" }] })),
        ],
        ingredients: vec![],
    },
    dest: "/path/to/photo_signed.jpg".into(),
    cert_path: "/path/to/cert.pem".into(),
    key_path:  "/path/to/key.pem".into(),
    alg: SigningAlg::Es256,
});
```

---

## Supported file formats

C2PA manifest embedding is supported for the formats listed below. The MIME type is inferred from the file extension.

| Extension | MIME type |
|-----------|-----------|
| jpg / jpeg | image/jpeg |
| png | image/png |
| mp4 / m4v | video/mp4 |
| mov | video/quicktime |
| avi | video/avi |
| pdf | application/pdf |
| tiff / tif | image/tiff |
| webp | image/webp |

---

## Further reading

- [docs/UI.md](docs/UI.md) — UI layout and design decisions
- [docs/dioxus.md](docs/dioxus.md) — Dioxus framework notes
- [docs/tauri.md](docs/tauri.md) — Tauri (vanilla JS) framework notes
- [C2PA specification](https://c2pa.org/specifications/specifications/2.1/specs/C2PA_Specification.html)
- [c2pa-rs crate](https://docs.rs/c2pa)
