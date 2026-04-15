use base64::{engine::general_purpose, Engine};
use c2pa::{Builder, Error, Manifest, ManifestAssertion, Reader, ValidationState};
pub use c2pa::SigningAlg;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

/// Overall C2PA validation outcome for a verified asset.
///
/// Returned as part of [`VerifyResult`] and maps directly to the c2pa-rs
/// [`ValidationState`] enum, with an extra `NoManifest` variant for files
/// that contain no C2PA data at all.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerifyValidationState {
    /// The signing certificate chains to a trusted root.
    Trusted,
    /// The manifest is cryptographically valid but the certificate is not
    /// in any known trust list.
    Valid,
    /// Validation failed (tampered content, expired certificate, etc.).
    Invalid,
    /// The file contains no embedded C2PA manifest store.
    NoManifest,
}

/// A single C2PA assertion extracted from a manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssertionSummary {
    /// Assertion label, e.g. `"c2pa.actions"` or `"stds.schema-org.CreativeWork"`.
    pub label: String,
    /// 1-based instance index when multiple assertions share the same label (v2.4).
    pub instance: usize,
    /// Full assertion payload deserialised into a JSON value.
    pub data: Value,
}

/// A single ingredient extracted from a manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngredientSummary {
    /// Human-readable title of the ingredient asset.
    pub title: Option<String>,
    /// IANA media type of the ingredient asset.
    pub format: Option<String>,
    /// Unique XMP instance ID (`xmpMM:InstanceID`) of the ingredient.
    pub instance_id: String,
    /// XMP document ID (`xmpMM:DocumentID`) of the ingredient, if present.
    pub document_id: Option<String>,
    /// Relationship of this ingredient to the containing asset
    /// (`"parentOf"`, `"componentOf"`, or `"inputTo"`).
    pub relationship: String,
    /// Label of the active manifest in the ingredient's own manifest store (v2.4).
    pub active_manifest: Option<String>,
    /// Optional human-readable description of the ingredient.
    pub description: Option<String>,
    /// Optional URI pointing to additional information about the ingredient.
    pub informational_uri: Option<String>,
    /// Assertion label used for this ingredient entry (e.g. `"c2pa.ingredient.v3"`) (v2.4).
    pub label: Option<String>,
    /// Full ingredient object serialised to JSON for display purposes.
    pub data: Value,
}

/// A summary of a single C2PA manifest within a manifest store.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestSummary {
    /// JUMBF label that uniquely identifies this manifest within the store.
    pub label: String,
    /// XMP instance ID of the asset this manifest describes.
    pub instance_id: String,
    /// Human-readable title of the asset.
    pub title: Option<String>,
    /// IANA media type of the asset. Present in claim v1 only (v2.4).
    pub format: Option<String>,
    /// Claim structure version: `1` = `c2pa.claim`, `2` = `c2pa.claim.v2` (v2.4).
    pub claim_version: Option<u8>,
    /// Free-text claim generator string. Present in claim v1 only (v2.4).
    pub claim_generator: Option<String>,
    /// Structured generator info list. Replaces `claim_generator` in claim v2 (v2.4).
    pub claim_generator_info: Option<Vec<Value>>,
    /// Issuer distinguished name from the signing certificate.
    pub issuer: Option<String>,
    /// Common name (CN) from the signing certificate (v2.4).
    pub common_name: Option<String>,
    /// Serial number of the signing certificate (v2.4).
    pub cert_serial_number: Option<String>,
    /// ISO-8601 signing timestamp, if present in the claim.
    pub signing_time: Option<String>,
    /// Signing algorithm identifier, e.g. `"Es256"` (v2.4).
    pub signature_alg: Option<String>,
    /// OCSP revocation status of the signing certificate: `true` = good (v2.4).
    pub revocation_status: Option<bool>,
    /// Base-64 data URI of the embedded thumbnail, ready for use in an `<img src>`.
    pub thumbnail_data_uri: Option<String>,
    /// All assertions found in this manifest.
    pub assertions: Vec<AssertionSummary>,
    /// All ingredients referenced by this manifest.
    pub ingredients: Vec<IngredientSummary>,
}

/// The complete verification result for a single asset file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifyResult {
    /// Absolute path of the file that was verified.
    pub file_path: String,
    /// Overall validation outcome.
    pub state: VerifyValidationState,
    /// Summary of the active (most recent) manifest, or `None` if absent.
    pub manifest: Option<ManifestSummary>,
    /// Summaries of all other manifests in the store (provenance chain).
    pub all_manifests: Vec<ManifestSummary>,
    /// Raw validation status objects from c2pa-rs, serialised to JSON.
    pub validation_statuses: Vec<Value>,
}

/// Convert a c2pa-rs [`Manifest`] into a [`ManifestSummary`] suitable for the UI.
fn summarize_manifest(m: &Manifest) -> ManifestSummary {
    let thumbnail_data_uri = m.thumbnail().map(|(format, bytes)| {
        let encoded = general_purpose::STANDARD.encode(bytes.as_ref());
        format!("data:{format};base64,{encoded}")
    });
    let sig = m.signature_info();
    ManifestSummary {
        label: m.label().unwrap_or("").to_string(),
        instance_id: m.instance_id().to_string(),
        title: m.title().map(str::to_string),
        format: m.format().map(str::to_string),
        claim_version: m.claim_version(),
        claim_generator: m.claim_generator().map(str::to_string),
        claim_generator_info: m.claim_generator_info.as_ref().map(|v| {
            v.iter()
                .map(|cgi| serde_json::to_value(cgi).unwrap_or(Value::Null))
                .collect()
        }),
        issuer: sig.and_then(|s| s.issuer.clone()),
        common_name: sig.and_then(|s| s.common_name.clone()),
        cert_serial_number: sig.and_then(|s| s.cert_serial_number.clone()),
        signing_time: sig.and_then(|s| s.time.clone()),
        signature_alg: sig.and_then(|s| s.alg.as_ref().map(|a| a.to_string())),
        revocation_status: sig.and_then(|s| s.revocation_status),
        thumbnail_data_uri,
        assertions: m
            .assertions()
            .iter()
            .map(|a: &ManifestAssertion| AssertionSummary {
                label: a.label().to_string(),
                instance: a.instance(),
                data: a.value().cloned().unwrap_or(Value::Null),
            })
            .collect(),
        ingredients: m
            .ingredients()
            .iter()
            .map(|i| IngredientSummary {
                title: i.title().map(str::to_string),
                format: i.format().map(str::to_string),
                instance_id: i.instance_id().to_string(),
                document_id: i.document_id().map(str::to_string),
                relationship: serde_json::to_value(i.relationship())
                    .unwrap_or_default()
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                active_manifest: i.active_manifest().map(str::to_string),
                description: i.description().map(str::to_string),
                informational_uri: i.informational_uri().map(str::to_string),
                label: i.label().map(str::to_string),
                data: serde_json::to_value(i).unwrap_or(Value::Null),
            })
            .collect(),
    }
}

/// Read the embedded C2PA manifest from `path` and return a verification summary.
///
/// Opens the file with [`Reader::from_file`], which also runs full manifest
/// validation (signature, hash, trust list).  If the file has no C2PA data
/// the returned [`VerifyResult::state`] is [`VerifyValidationState::NoManifest`].
///
/// This function never panics; all errors are captured in the result.
pub fn verify_embedded_manifest(path: &str) -> VerifyResult {
    info!(target: "model::verify", "Verifying: {path}");
    match Reader::from_file(path) {
        Err(Error::JumbfNotFound) => {
            info!(target: "model::verify", "No C2PA manifest found in {path}");
            VerifyResult {
                file_path: path.to_string(),
                state: VerifyValidationState::NoManifest,
                manifest: None,
                all_manifests: vec![],
                validation_statuses: vec![],
            }
        }
        Err(e) => {
            error!(target: "model::verify", "Failed to read manifest: {e}");
            VerifyResult {
                file_path: path.to_string(),
                state: VerifyValidationState::Invalid,
                manifest: None,
                all_manifests: vec![],
                validation_statuses: vec![],
            }
        }
        Ok(reader) => {
            let state = match reader.validation_state() {
                ValidationState::Trusted => { info!(target: "model::verify", "Validation: TRUSTED"); VerifyValidationState::Trusted }
                ValidationState::Valid   => { info!(target: "model::verify", "Validation: VALID");   VerifyValidationState::Valid }
                ValidationState::Invalid => { warn!(target: "model::verify", "Validation: INVALID"); VerifyValidationState::Invalid }
            };
            let manifest = reader.active_manifest().map(summarize_manifest);
            if let Some(m) = &manifest {
                debug!(target: "model::verify", "Active manifest: {} ({} assertions, {} ingredients)",
                    m.label, m.assertions.len(), m.ingredients.len());
            }
            let active_label = manifest.as_ref().map(|m| m.label.clone());
            let all_manifests = reader
                .iter_manifests()
                .filter(|m| Some(m.label().unwrap_or("")) != active_label.as_deref())
                .map(summarize_manifest)
                .collect();
            let validation_statuses = reader
                .validation_status()
                .map(|statuses| {
                    statuses
                        .iter()
                        .map(|s| serde_json::to_value(s).unwrap_or(Value::Null))
                        .collect()
                })
                .unwrap_or_default();
            VerifyResult {
                file_path: path.to_string(),
                state,
                manifest,
                all_manifests,
                validation_statuses,
            }
        }
    }
}

/// A file to embed as an ingredient in a manifest.
#[derive(Clone, Debug)]
pub struct IngredientEntry {
    /// Absolute path to the ingredient file.
    pub path: String,
    /// C2PA relationship of this ingredient to the parent asset.
    /// One of `"parentOf"`, `"componentOf"`, or `"inputTo"`.
    pub relationship: String,
    /// Optional override title for the ingredient (defaults to the filename).
    pub title: Option<String>,
}

/// Parameters shared by both [`add_manifest`] and [`sign_asset`].
pub struct ManifestParams {
    /// Absolute path of the source asset file.
    pub source: String,
    /// Optional manifest title; if `None` the filename is used.
    pub title: Option<String>,
    /// Override MIME type for the asset.  When `None` the type is inferred
    /// from the file extension.
    pub format: Option<String>,
    /// Assertions to embed, as `(label, data)` pairs where `data` is the
    /// JSON payload for that assertion type.
    pub assertions: Vec<(String, Value)>,
    /// Ingredient files to reference in the manifest.
    pub ingredients: Vec<IngredientEntry>,
}

/// Parameters for the [`sign_asset`] operation.
pub struct SignParams {
    /// Manifest content and source asset.
    pub manifest: ManifestParams,
    /// Absolute path where the signed output file should be written.
    pub dest: String,
    /// Absolute path to the signing certificate chain file (PEM).
    pub cert_path: String,
    /// Absolute path to the private key file (PEM).
    pub key_path: String,
    /// Signing algorithm to use.
    pub alg: SigningAlg,
}

/// Infer the IANA media type for `source` from its file extension.
/// Falls back to `"application/octet-stream"` for unrecognised extensions.
fn ext_to_mime(source: &str) -> &'static str {
    match Path::new(source)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png")                => "image/png",
        Some("mp4") | Some("m4v") => "video/mp4",
        Some("mov")                => "video/quicktime",
        Some("avi")                => "video/avi",
        Some("pdf")                => "application/pdf",
        Some("tiff") | Some("tif") => "image/tiff",
        Some("webp")               => "image/webp",
        _                          => "application/octet-stream",
    }
}

/// Build a c2pa-rs [`Builder`] from [`ManifestParams`].
///
/// Constructs the manifest JSON, loads each ingredient from disk, and
/// attaches it with the specified relationship.  Returns an error string
/// if any ingredient file cannot be opened.
fn build_builder(p: &ManifestParams) -> Result<Builder, String> {
    let format = p.format.as_deref().unwrap_or_else(|| ext_to_mime(&p.source)).to_string();

    let assertion_values: Vec<Value> = p.assertions.iter()
        .map(|(label, data)| json!({ "label": label, "data": data }))
        .collect();

    let mut def = json!({ "format": format, "assertions": assertion_values });
    if let Some(t) = &p.title {
        def["title"] = json!(t);
    }

    let mut builder = Builder::from_json(&def.to_string())
        .map_err(|e| format!("Failed to create manifest builder: {e}"))?;

    for ing_entry in &p.ingredients {
        let mut ingredient = c2pa::Ingredient::from_file(&ing_entry.path)
            .map_err(|e| format!("Failed to load ingredient '{}': {e}", ing_entry.path))?;
        if let Some(t) = &ing_entry.title {
            ingredient.set_title(t);
        }
        let rel = match ing_entry.relationship.as_str() {
            "parentOf" => c2pa::Relationship::ParentOf,
            "inputTo"  => c2pa::Relationship::InputTo,
            _          => c2pa::Relationship::ComponentOf,
        };
        ingredient.set_relationship(rel);
        builder.add_ingredient(ingredient);
    }

    Ok(builder)
}

/// Export a C2PA manifest archive (`.c2pa`) for `params.source` without
/// embedding or signing it.
///
/// This is useful for inspecting or distributing the manifest separately
/// from the asset.  The archive can later be attached to an asset with
/// standard C2PA tools.
///
/// # Errors
/// Returns an error string if the builder cannot be constructed, the
/// output file cannot be created, or the archive serialisation fails.
pub fn add_manifest(params: ManifestParams, dest: String) -> Result<String, String> {
    info!(target: "model::sign", "Exporting manifest archive: {dest}");
    debug!(target: "model::sign", "Source: {}, assertions: {}", params.source, params.assertions.len());
    let mut builder = build_builder(&params)?;
    let mut file = std::fs::File::create(&dest)
        .map_err(|e| format!("Cannot create file '{dest}': {e}"))?;
    builder.to_archive(&mut file)
        .map_err(|e| { error!(target: "model::sign", "Archive failed: {e}"); format!("Failed to write manifest archive: {e}") })?;
    info!(target: "model::sign", "Manifest archive written: {dest}");
    Ok(dest)
}

/// Sign `params.manifest.source` with the provided certificate and key,
/// embedding the C2PA manifest, and write the result to `params.dest`.
///
/// The source file is read-only; a new signed copy is created at `dest`.
///
/// # Errors
/// Returns an error string if the builder cannot be constructed, the signer
/// cannot be initialised from the provided certificate/key files, or the
/// signing operation itself fails.
pub fn sign_asset(params: SignParams) -> Result<String, String> {
    info!(target: "model::sign", "Signing asset: {} → {}", params.manifest.source, params.dest);
    debug!(target: "model::sign", "Algorithm: {:?}, assertions: {}, ingredients: {}",
        params.alg, params.manifest.assertions.len(), params.manifest.ingredients.len());
    let mut builder = build_builder(&params.manifest)?;

    let signer = c2pa::create_signer::from_files(
        &params.cert_path, &params.key_path, params.alg, None,
    ).map_err(|e| { error!(target: "model::sign", "Signer load failed: {e}"); format!("Failed to load signer: {e}") })?;

    info!(target: "model::sign", "Signer loaded, embedding manifest…");
    builder
        .sign_file(signer.as_ref(), &params.manifest.source, &params.dest)
        .map_err(|e| { error!(target: "model::sign", "Sign failed: {e}"); format!("Signing failed: {e}") })?;

    info!(target: "model::sign", "Signed asset written: {}", params.dest);
    Ok(params.dest)
}
