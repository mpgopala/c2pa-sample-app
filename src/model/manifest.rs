use base64::{engine::general_purpose, Engine};
use c2pa::{Error, Manifest, ManifestAssertion, Reader, ValidationState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
    match Reader::from_file(path) {
        Err(Error::JumbfNotFound) => VerifyResult {
            file_path: path.to_string(),
            state: VerifyValidationState::NoManifest,
            manifest: None,
            all_manifests: vec![],
            validation_statuses: vec![],
        },
        Err(_) => VerifyResult {
            file_path: path.to_string(),
            state: VerifyValidationState::Invalid,
            manifest: None,
            all_manifests: vec![],
            validation_statuses: vec![],
        },
        Ok(reader) => {
            let state = match reader.validation_state() {
                ValidationState::Trusted => VerifyValidationState::Trusted,
                ValidationState::Valid => VerifyValidationState::Valid,
                ValidationState::Invalid => VerifyValidationState::Invalid,
            };
            let manifest = reader.active_manifest().map(summarize_manifest);
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
