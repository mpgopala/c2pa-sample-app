# Model Layer — Class Diagram

```mermaid
classDiagram

    %% ── manifest module ────────────────────────────────────────────────────

    class VerifyValidationState {
        <<enumeration>>
        Trusted
        Valid
        Invalid
        NoManifest
    }

    class AssertionSummary {
        +label: String
        +instance: usize
        +data: Value
    }

    class IngredientSummary {
        +title: Option~String~
        +format: Option~String~
        +instance_id: String
        +document_id: Option~String~
        +relationship: String
        +active_manifest: Option~String~
        +description: Option~String~
        +informational_uri: Option~String~
        +label: Option~String~
        +data: Value
    }

    class ManifestSummary {
        +label: String
        +instance_id: String
        +title: Option~String~
        +format: Option~String~
        +claim_version: Option~u8~
        +claim_generator: Option~String~
        +claim_generator_info: Option~Vec~Value~~
        +issuer: Option~String~
        +common_name: Option~String~
        +cert_serial_number: Option~String~
        +signing_time: Option~String~
        +signature_alg: Option~String~
        +revocation_status: Option~bool~
        +thumbnail_data_uri: Option~String~
        +assertions: Vec~AssertionSummary~
        +ingredients: Vec~IngredientSummary~
    }

    class VerifyResult {
        +file_path: String
        +state: VerifyValidationState
        +manifest: Option~ManifestSummary~
        +all_manifests: Vec~ManifestSummary~
        +validation_statuses: Vec~Value~
        +verify_embedded_manifest(path: &str)$ VerifyResult
    }

    %% ── recents module ────────────────────────────────────────────────────

    class RecentEntry {
        +path: String
        +name: String
        +timestamp: u64
    }

    class RecentsStore {
        <<module: recents>>
        +load_recents()$ Vec~RecentEntry~
        +push_recent(path: &str, entries: &mut Vec~RecentEntry~)$
        -save_recents(entries: &[RecentEntry])$
        -recents_path()$ PathBuf
        -MAX_RECENTS: usize = 10
    }

    %% ── relationships ─────────────────────────────────────────────────────

    VerifyResult "1" --> "1" VerifyValidationState : state
    VerifyResult "1" --> "0..1" ManifestSummary     : manifest (active)
    VerifyResult "1" --> "0..*" ManifestSummary     : all_manifests

    ManifestSummary "1" --> "0..*" AssertionSummary  : assertions
    ManifestSummary "1" --> "0..*" IngredientSummary : ingredients

    RecentsStore ..> RecentEntry : creates / manages
```

## Notes

| Type | Module | Role |
|---|---|---|
| `VerifyValidationState` | `manifest` | Enum mapping the c2pa `ValidationState` to a serialisable form understood by the UI. |
| `AssertionSummary` | `manifest` | Flattened view of one assertion inside a manifest (label, 1-based instance index, raw JSON payload). |
| `IngredientSummary` | `manifest` | Flattened view of one ingredient assertion, including the JUMBF assertion `label` used for assertion→ingredient navigation. |
| `ManifestSummary` | `manifest` | Complete summary of one C2PA manifest as parsed against spec v2.4: claim metadata, signature details, assertions, and ingredients. |
| `VerifyResult` | `manifest` | Top-level output of `verify_embedded_manifest`. Carries the active manifest, all other manifests in the store, the validation state, and raw validation statuses. |
| `RecentEntry` | `recents` | One entry in the persisted recently-opened-files list. |
| `RecentsStore` | `recents` | Module-level functions that load, mutate, and persist the recents list to `~/.c2pa-tool/recents.json`. Represented as a class for diagram clarity; there is no struct. |
