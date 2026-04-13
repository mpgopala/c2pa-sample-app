use c2pa_sample_app::model::manifest::{
    add_manifest, sign_asset, verify_embedded_manifest,
    IngredientEntry, ManifestParams, SignParams, SigningAlg, VerifyResult,
};
use c2pa_sample_app::model::recents::{load_recents, push_recent, RecentEntry};
use c2pa_sample_app::model::signer_prefs::{load_signer_prefs, save_signer_prefs, SignerPrefs};
use crate::logger::{drain_logs, LogEntry};
use serde::Deserialize;
use serde_json::Value;

// ── DTOs ──────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IngredientEntryDto {
    pub path: String,
    pub relationship: String,
    pub title: Option<String>,
}

#[derive(Deserialize)]
pub struct ManifestParamsDto {
    pub source: String,
    pub title: Option<String>,
    pub format: Option<String>,
    pub assertions: Vec<(String, Value)>,
    pub ingredients: Vec<IngredientEntryDto>,
}

#[derive(Deserialize)]
pub struct SignParamsDto {
    pub manifest: ManifestParamsDto,
    pub dest: String,
    pub cert_path: String,
    pub key_path: String,
    pub alg: String,
}

#[derive(Deserialize)]
pub struct SignerPrefsDto {
    pub cert_path: String,
    pub key_path: String,
    pub alg: String,
}

impl From<IngredientEntryDto> for IngredientEntry {
    fn from(d: IngredientEntryDto) -> Self {
        IngredientEntry { path: d.path, relationship: d.relationship, title: d.title }
    }
}

impl From<ManifestParamsDto> for ManifestParams {
    fn from(d: ManifestParamsDto) -> Self {
        ManifestParams {
            source: d.source,
            title: d.title,
            format: d.format,
            assertions: d.assertions,
            ingredients: d.ingredients.into_iter().map(Into::into).collect(),
        }
    }
}

fn parse_alg(s: &str) -> SigningAlg {
    match s {
        "Es384"   => SigningAlg::Es384,
        "Es512"   => SigningAlg::Es512,
        "Ps256"   => SigningAlg::Ps256,
        "Ps384"   => SigningAlg::Ps384,
        "Ps512"   => SigningAlg::Ps512,
        "Ed25519" => SigningAlg::Ed25519,
        _         => SigningAlg::Es256,
    }
}

// ── commands ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn verify_asset(path: String) -> VerifyResult {
    verify_embedded_manifest(&path)
}

#[tauri::command]
pub fn add_manifest_cmd(params: ManifestParamsDto, dest: String) -> Result<String, String> {
    add_manifest(params.into(), dest)
}

#[tauri::command]
pub fn sign_asset_cmd(params: SignParamsDto) -> Result<String, String> {
    sign_asset(SignParams {
        manifest: params.manifest.into(),
        dest: params.dest,
        cert_path: params.cert_path,
        key_path: params.key_path,
        alg: parse_alg(&params.alg),
    })
}

#[tauri::command]
pub fn load_recents_cmd() -> Vec<RecentEntry> {
    load_recents()
}

#[tauri::command]
pub fn push_recent_cmd(path: String) -> Vec<RecentEntry> {
    let mut entries = load_recents();
    push_recent(&path, &mut entries);
    entries
}

#[tauri::command]
pub fn drain_logs_cmd() -> Vec<LogEntry> {
    drain_logs()
}

#[tauri::command]
pub fn load_signer_prefs_cmd() -> SignerPrefs {
    load_signer_prefs()
}

#[tauri::command]
pub fn save_signer_prefs_cmd(prefs: SignerPrefsDto) {
    save_signer_prefs(&SignerPrefs {
        cert_path: prefs.cert_path,
        key_path: prefs.key_path,
        alg: prefs.alg,
    });
}
