# Tier 1 Feature Requirements

## Can sign asset type, including unsupported via external manifest generation

**Key APIs:** `Builder`, `ManifestDefinition`, `Builder::with_definition()`, `Builder::save_to_stream()`, `Builder::save_to_file()`, `Builder::set_no_embed()`

The sample app demonstrates signing various asset types by using the `Builder` to create manifests from JSON/TOML definitions. External manifest generation is supported via `Builder::set_no_embed()` which creates a separate manifest file instead of embedding it. Supported MIME types can be queried with `Builder::supported_mime_types()`. For unsupported formats, the app can use `Builder::composed_manifest()` to wrap the manifest appropriately for the target format, enabling broad asset type coverage.

## Can sign fragmented content

**Key APIs:** `Builder::add_ingredient_from_stream()`, `Reader::from_fragment()`, `Reader::with_fragment()`, `Reader::from_fragmented_files()`

Fragmented content signing uses `Builder::add_ingredient_from_stream()` to incorporate fragments as ingredients before signing. The sample app demonstrates handling MP4 fragmentation patterns where initial segments and fragment files are processed together. Using `BuilderIntent::Edit` automatically captures the parent ingredient from fragmented sources. The builder's stream-based API naturally accommodates varying fragment sizes and chunked data patterns.

## Can read and verify any content with manifests, including external and remote manifests

**Key APIs:** `Reader`, `Reader::from_file()`, `Reader::with_file()`, `Reader::from_stream()`, `Reader::with_manifest_data_and_stream()`, `fetch_remote_manifests` feature

The sample app uses `Reader` to handle embedded, external, and remote manifests seamlessly. When `fetch_remote_manifests` feature is enabled, `Reader::from_file()` automatically fetches remote manifests specified in the asset. For external manifests, the Reader automatically checks for sidecar `.c2pa` files. The `with_manifest_data_and_stream()` methods allow explicit validation of remote or external manifest data against the source content stream.

## Fragmented content validation

**Key APIs:** `Reader::from_fragment()`, `Reader::with_fragment()`, `Reader::validation_state()`, `Reader::validation_results()`, `ValidationState` enum

The sample app validates fragmented content by loading initial segments and fragments together using `Reader::from_fragment()` or `Reader::with_fragment()` which automatically assembles and validates across fragment boundaries. Validation results are accessed through `Reader::validation_state()` (returns overall `ValidationState` enum: Unsigned, Unverifiable, Tampered, Mismatch, Verified, etc.) and `Reader::validation_results()` for detailed per-assertion validation status. This ensures security is maintained across fragment chains.

## Ability to load settings

**Key APIs:** `Context::with_settings()`, `Settings` struct, JSON/TOML configuration support

The sample app creates a `Context` with settings loaded from JSON strings, TOML files, or configuration objects via `Context::new().with_settings(config)`. Settings control verification behavior, trust configuration, signer setup, HTTP resolution options, and manifest handling. The flexible `IntoSettings` trait accepts multiple input formats including file paths, making integration with application configuration systems straightforward.

## Ability to load trust lists

**Key APIs:** `Settings` with trust configuration, `Context::with_settings()`, `Reader::validation_state()`

Trust lists are configured in settings through the "trust" section, specifying PEM-encoded CA certificates and anchors. The sample app loads official C2PA trust lists and applies them by configuring a `Context` with these settings before creating `Reader` instances. During manifest validation, the trust configuration is automatically applied. The validation result indicates whether certificates are trusted based on the loaded trust list.

## Ability to load manifest definitions

**Key APIs:** `ManifestDefinition`, `Builder::with_definition()`, JSON/TOML deserialization, `ManifestDefinition::default()`

The sample app loads manifest definitions from JSON strings via `ManifestDefinition::try_from()` or directly using `Builder::from_json()` and `Builder::with_definition()`. Definitions can specify ingredients, assertions, claim generator info, titles, and format constraints. The structured definition API allows the app to validate that manifests conform to expected schemas before signing, ensuring consistent structure across different use cases.

## Ingredient composition

**Key APIs:** `Ingredient` struct, `Builder::add_ingredient()`, `Builder::add_ingredient_from_stream()`, `Builder::add_ingredient_from_reader()`, `Manifest::ingredients()`

The sample app demonstrates ingredient chains by adding ingredients during manifest construction using `Builder::add_ingredient_from_stream()` (which accepts streams of ingredient assets) or `Builder::add_ingredient_from_reader()` (which extracts ingredients from previously-read manifests). The `Ingredient` struct includes relationship types and metadata. When reading signed assets, `Reader::active_manifest()` followed by `Manifest::ingredients()` retrieves the full chain, demonstrating content provenance and attribution tracking.
