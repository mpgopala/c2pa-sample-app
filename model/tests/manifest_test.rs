//! Integration tests for `model::manifest`.
//!
//! Covers error and no-manifest paths of `verify_embedded_manifest`.
//! Sign/verify round-trip with a real cert chain is intentionally out of
//! scope here — that requires fixture cert+key plumbing tracked separately.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as B64;

use model::manifest::{verify_embedded_manifest, VerifyValidationState};

/// 67-byte 1×1 transparent PNG, no C2PA payload.
const BLANK_PNG_B64: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII=";

fn write_blank_png(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("c2pa-tool-test-manifest-{}-{label}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("blank.png");
    let bytes = B64.decode(BLANK_PNG_B64).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    path
}

#[test]
fn verify_nonexistent_file_yields_invalid_with_no_manifest() {
    let result = verify_embedded_manifest("/definitely/does/not/exist/asset.jpg");
    assert_eq!(result.state, VerifyValidationState::Invalid);
    assert!(result.manifest.is_none());
    assert!(result.all_manifests.is_empty());
    assert_eq!(result.file_path, "/definitely/does/not/exist/asset.jpg");
}

#[test]
fn verify_png_without_manifest_yields_no_manifest_state() {
    let path = write_blank_png("no-manifest");
    let result = verify_embedded_manifest(path.to_str().unwrap());
    assert_eq!(
        result.state,
        VerifyValidationState::NoManifest,
        "blank PNG must report NoManifest, got {:?}",
        result.state
    );
    assert!(result.manifest.is_none());
    assert!(result.all_manifests.is_empty());
    assert!(result.validation_statuses.is_empty());
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}

#[test]
fn verify_result_preserves_input_path() {
    let path = write_blank_png("path-preserved");
    let path_str = path.to_str().unwrap();
    let result = verify_embedded_manifest(path_str);
    assert_eq!(result.file_path, path_str);
    let _ = std::fs::remove_dir_all(path.parent().unwrap());
}
