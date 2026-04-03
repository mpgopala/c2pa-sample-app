use base64::{engine::general_purpose, Engine};
use c2pa::{Builder, Error, Manifest, ManifestAssertion, Reader, ValidationState};
pub use c2pa::SigningAlg;
use tracing::{debug, error, info, warn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerifyValidationState {
    Trusted,
    Valid,
    Invalid,
    NoManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssertionSummary {
    pub label: String,
    /// 1-based instance index when multiple assertions share the same label (v2.4).
    pub instance: usize,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngredientSummary {
    pub title: Option<String>,
    pub format: Option<String>,
    pub instance_id: String,
    pub document_id: Option<String>,
    pub relationship: String,
    /// Label of the active manifest in the ingredient's own manifest store (v2.4).
    pub active_manifest: Option<String>,
    pub description: Option<String>,
    pub informational_uri: Option<String>,
    /// Assertion label used for this ingredient (e.g. "c2pa.ingredient.v3") (v2.4).
    pub label: Option<String>,
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ManifestSummary {
    pub label: String,
    pub instance_id: String,
    pub title: Option<String>,
    /// IANA media type of the asset. Present in claim v1 only (v2.4).
    pub format: Option<String>,
    /// Claim structure version: 1 = c2pa.claim, 2 = c2pa.claim.v2 (v2.4).
    pub claim_version: Option<u8>,
    /// Free-text claim generator string. Present in claim v1 only (v2.4).
    pub claim_generator: Option<String>,
    /// Structured generator info list. Replaces claim_generator in claim v2 (v2.4).
    pub claim_generator_info: Option<Vec<Value>>,
    pub issuer: Option<String>,
    /// Common name (CN) from the signing certificate (v2.4).
    pub common_name: Option<String>,
    /// Serial number of the signing certificate (v2.4).
    pub cert_serial_number: Option<String>,
    pub signing_time: Option<String>,
    /// Signing algorithm identifier (v2.4).
    pub signature_alg: Option<String>,
    /// OCSP revocation status of the signing certificate (v2.4).
    pub revocation_status: Option<bool>,
    pub thumbnail_data_uri: Option<String>,
    pub assertions: Vec<AssertionSummary>,
    pub ingredients: Vec<IngredientSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResult {
    pub file_path: String,
    pub state: VerifyValidationState,
    pub manifest: Option<ManifestSummary>,
    pub all_manifests: Vec<ManifestSummary>,
    pub validation_statuses: Vec<Value>,
}

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

/// Read the embedded C2PA manifest from `path` and return a summary.
/// Returns `NoManifest` if the file has no embedded manifest.
pub fn verify_embedded_manifest(path: &str) -> VerifyResult {
    info!(target: "c2pa_tool::verify", "Verifying: {path}");
    match Reader::from_file(path) {
        Err(Error::JumbfNotFound) => {
            info!(target: "c2pa_tool::verify", "No C2PA manifest found in {path}");
            VerifyResult {
                file_path: path.to_string(),
                state: VerifyValidationState::NoManifest,
                manifest: None,
                all_manifests: vec![],
                validation_statuses: vec![],
            }
        }
        Err(e) => {
            error!(target: "c2pa_tool::verify", "Failed to read manifest: {e}");
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
                ValidationState::Trusted => { info!(target: "c2pa_tool::verify", "Validation: TRUSTED"); VerifyValidationState::Trusted }
                ValidationState::Valid   => { info!(target: "c2pa_tool::verify", "Validation: VALID");   VerifyValidationState::Valid }
                ValidationState::Invalid => { warn!(target: "c2pa_tool::verify", "Validation: INVALID"); VerifyValidationState::Invalid }
            };
            let manifest = reader.active_manifest().map(summarize_manifest);
            if let Some(m) = &manifest {
                debug!(target: "c2pa_tool::verify", "Active manifest: {} ({} assertions, {} ingredients)",
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

/// An ingredient to embed in the manifest.
#[derive(Clone, Debug)]
pub struct IngredientEntry {
    pub path: String,
    /// "parentOf" | "componentOf" | "inputTo"
    pub relationship: String,
    pub title: Option<String>,
}

/// Parameters common to both actions (add-manifest and sign-asset).
pub struct ManifestParams {
    pub source: String,
    pub title: Option<String>,
    pub format: Option<String>,
    /// (label, data) pairs — data is the assertion payload JSON value.
    pub assertions: Vec<(String, Value)>,
    pub ingredients: Vec<IngredientEntry>,
}

/// Parameters for signing an asset with a C2PA manifest.
pub struct SignParams {
    pub manifest: ManifestParams,
    pub dest: String,
    pub cert_path: String,
    pub key_path: String,
    pub alg: SigningAlg,
}

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

/// Export a C2PA manifest archive (.c2pa) without signing the asset.
/// `dest` should end in `.c2pa`.
/// Returns the destination path on success.
pub fn add_manifest(params: ManifestParams, dest: String) -> Result<String, String> {
    info!(target: "c2pa_tool::sign", "Exporting manifest archive: {dest}");
    debug!(target: "c2pa_tool::sign", "Source: {}, assertions: {}", params.source, params.assertions.len());
    let mut builder = build_builder(&params)?;
    let mut file = std::fs::File::create(&dest)
        .map_err(|e| format!("Cannot create file '{dest}': {e}"))?;
    builder.to_archive(&mut file)
        .map_err(|e| { error!(target: "c2pa_tool::sign", "Archive failed: {e}"); format!("Failed to write manifest archive: {e}") })?;
    info!(target: "c2pa_tool::sign", "Manifest archive written: {dest}");
    Ok(dest)
}

/// Sign `params.manifest.source` and write the signed output to `params.dest`.
/// Returns the destination path on success.
pub fn sign_asset(params: SignParams) -> Result<String, String> {
    info!(target: "c2pa_tool::sign", "Signing asset: {} → {}", params.manifest.source, params.dest);
    debug!(target: "c2pa_tool::sign", "Algorithm: {:?}, assertions: {}, ingredients: {}",
        params.alg, params.manifest.assertions.len(), params.manifest.ingredients.len());
    let mut builder = build_builder(&params.manifest)?;

    let signer = c2pa::create_signer::from_files(
        &params.cert_path, &params.key_path, params.alg, None,
    ).map_err(|e| { error!(target: "c2pa_tool::sign", "Signer load failed: {e}"); format!("Failed to load signer: {e}") })?;

    info!(target: "c2pa_tool::sign", "Signer loaded, embedding manifest…");
    builder
        .sign_file(signer.as_ref(), &params.manifest.source, &params.dest)
        .map_err(|e| { error!(target: "c2pa_tool::sign", "Sign failed: {e}"); format!("Signing failed: {e}") })?;

    info!(target: "c2pa_tool::sign", "Signed asset written: {}", params.dest);
    Ok(params.dest)
}
