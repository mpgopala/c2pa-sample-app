use c2pa_model::manifest::{add_manifest, sign_asset, IngredientEntry, ManifestParams, SignParams, SigningAlg};
use c2pa_model::preferences::{load_preferences, save_preferences, Preferences};
use dioxus::prelude::*;
use serde_json::{json, Value};
use tracing::{debug, info};

// ── types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Debug)]
struct AssertionEntry {
    label: String,
    data: Value,
}

#[derive(Clone, PartialEq)]
enum SignResult {
    Idle,
    Success(String),
    Error(String),
}

// ── helpers ───────────────────────────────────────────────────────────────────

/// e.g. `/tmp/photo.jpg` → `/tmp/photo_signed.jpg`
fn derive_signed_dest(source: &str) -> String {
    let p = std::path::Path::new(source);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    let dir = p.parent().and_then(|d| d.to_str()).unwrap_or(".");
    if ext.is_empty() { format!("{dir}/{stem}_signed") } else { format!("{dir}/{stem}_signed.{ext}") }
}

/// e.g. `/tmp/photo.jpg` → `/tmp/photo.c2pa`
fn derive_manifest_dest(source: &str) -> String {
    let p = std::path::Path::new(source);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
    let dir = p.parent().and_then(|d| d.to_str()).unwrap_or(".");
    format!("{dir}/{stem}.c2pa")
}

fn alg_label(alg: &SigningAlg) -> &'static str {
    match alg {
        SigningAlg::Es256  => "ES256 (ECDSA / SHA-256)",
        SigningAlg::Es384  => "ES384 (ECDSA / SHA-384)",
        SigningAlg::Es512  => "ES512 (ECDSA / SHA-512)",
        SigningAlg::Ps256  => "PS256 (RSA-PSS / SHA-256)",
        SigningAlg::Ps384  => "PS384 (RSA-PSS / SHA-384)",
        SigningAlg::Ps512  => "PS512 (RSA-PSS / SHA-512)",
        SigningAlg::Ed25519 => "Ed25519",
    }
}

/// Default `Value` payload for a newly-added assertion label.
fn default_data_for(label: &str) -> Value {
    match label {
        "c2pa.actions" => json!({
            "actions": [{ "action": "c2pa.created" }]
        }),
        "c2pa.training-mining" => json!({
            "use_train": false,
            "use_mine": false
        }),
        "stds.schema-org.CreativeWork" => json!({
            "@context": "http://schema.org/",
            "@type": "CreativeWork",
            "author": [{ "@type": "Person", "name": "" }],
            "copyrightNotice": ""
        }),
        _ => json!({}),
    }
}

const ALGS: &[SigningAlg] = &[
    SigningAlg::Es256, SigningAlg::Es384, SigningAlg::Es512,
    SigningAlg::Ps256, SigningAlg::Ps384, SigningAlg::Ps512,
    SigningAlg::Ed25519,
];

const PRESET_ASSERTIONS: &[&str] = &[
    "c2pa.actions",
    "c2pa.training-mining",
    "stds.schema-org.CreativeWork",
    "c2pa.hash.data",
    "c2pa.soft-binding",
];

const ACTION_TYPES: &[&str] = &[
    "c2pa.created", "c2pa.edited", "c2pa.published", "c2pa.converted",
    "c2pa.repackaged", "c2pa.transcoded", "c2pa.resized", "c2pa.color_adjustments",
    "c2pa.cropped", "c2pa.drawing", "c2pa.filtered", "c2pa.placed",
];

const DIGITAL_SOURCE_TYPES: &[(&str, &str)] = &[
    ("", "— none —"),
    ("http://cv.iptc.org/newscodes/digitalsourcetype/algorithmicMedia",   "Algorithmic Media (AI)"),
    ("http://cv.iptc.org/newscodes/digitalsourcetype/trainedAlgorithmicMedia", "Trained Algorithmic Media"),
    ("http://cv.iptc.org/newscodes/digitalsourcetype/digitalCapture",     "Digital Capture"),
    ("http://cv.iptc.org/newscodes/digitalsourcetype/digitalArt",         "Digital Art"),
    ("http://cv.iptc.org/newscodes/digitalsourcetype/compositeWithTrainedAlgorithmicMedia", "Composite + AI"),
];

const RELATIONSHIPS: &[(&str, &str)] = &[
    ("componentOf", "Component Of"),
    ("parentOf",    "Parent Of"),
    ("inputTo",     "Input To"),
];

// ── assertion editors ─────────────────────────────────────────────────────────

/// Editor for c2pa.actions
#[component]
fn ActionsEditor(assertions: Signal<Vec<AssertionEntry>>, idx: usize) -> Element {
    let actions = {
        let a = assertions.read();
        a[idx].data["actions"].as_array().cloned().unwrap_or_default()
    };

    let mut update_action = move |ai: usize, field: &str, value: Value| {
        let mut a = assertions.write();
        if let Some(arr) = a[idx].data["actions"].as_array_mut() {
            if let Some(entry) = arr.get_mut(ai) {
                entry[field] = value;
            }
        }
    };

    let add_action = move |_| {
        let mut a = assertions.write();
        if let Some(arr) = a[idx].data["actions"].as_array_mut() {
            arr.push(json!({ "action": "c2pa.created" }));
        }
    };

    let mut remove_action = move |ai: usize| {
        let mut a = assertions.write();
        if let Some(arr) = a[idx].data["actions"].as_array_mut() {
            if arr.len() > 1 { arr.remove(ai); }
        }
    };

    rsx! {
        div { class: "assertion-editor",
            for (ai, action) in actions.iter().enumerate() {
                {
                    let current_action = action["action"].as_str().unwrap_or("c2pa.created").to_string();
                    let current_dst = action["digitalSourceType"].as_str().unwrap_or("").to_string();
                    let ai_remove = ai;
                    rsx! {
                        div { class: "action-row",
                            select {
                                class: "field-select",
                                onchange: move |e| update_action(ai, "action", json!(e.value())),
                                for at in ACTION_TYPES {
                                    option {
                                        value: "{at}",
                                        selected: *at == current_action.as_str(),
                                        "{at}"
                                    }
                                }
                            }
                            select {
                                class: "field-select",
                                onchange: move |e| {
                                    let v = e.value();
                                    let mut a = assertions.write();
                                    if let Some(arr) = a[idx].data["actions"].as_array_mut() {
                                        if let Some(entry) = arr.get_mut(ai) {
                                            if v.is_empty() {
                                                entry.as_object_mut().map(|o| o.remove("digitalSourceType"));
                                            } else {
                                                entry["digitalSourceType"] = json!(v);
                                            }
                                        }
                                    }
                                },
                                for (val, lbl) in DIGITAL_SOURCE_TYPES {
                                    option {
                                        value: "{val}",
                                        selected: *val == current_dst.as_str(),
                                        "{lbl}"
                                    }
                                }
                            }
                            button {
                                class: "btn btn-sm btn-danger",
                                disabled: actions.len() <= 1,
                                onclick: move |_| remove_action(ai_remove),
                                "✕"
                            }
                        }
                    }
                }
            }
            div { class: "add-row",
                button { class: "btn btn-sm", onclick: add_action, "+ Action" }
            }
        }
    }
}

/// Editor for c2pa.training-mining
#[component]
fn TrainingMiningEditor(assertions: Signal<Vec<AssertionEntry>>, idx: usize) -> Element {
    let use_train = assertions.read()[idx].data["use_train"].as_bool().unwrap_or(false);
    let use_mine  = assertions.read()[idx].data["use_mine"].as_bool().unwrap_or(false);

    rsx! {
        div { class: "assertion-editor",
            label { class: "checkbox-row",
                input {
                    r#type: "checkbox",
                    checked: use_train,
                    onchange: move |e| {
                        assertions.write()[idx].data["use_train"] = json!(e.checked());
                    }
                }
                " Allow AI Training"
            }
            label { class: "checkbox-row",
                input {
                    r#type: "checkbox",
                    checked: use_mine,
                    onchange: move |e| {
                        assertions.write()[idx].data["use_mine"] = json!(e.checked());
                    }
                }
                " Allow Data Mining"
            }
        }
    }
}

/// Editor for stds.schema-org.CreativeWork
#[component]
fn CreativeWorkEditor(assertions: Signal<Vec<AssertionEntry>>, idx: usize) -> Element {
    let author = {
        let a = assertions.read();
        a[idx].data["author"].as_array()
            .and_then(|arr| arr.first())
            .and_then(|a| a["name"].as_str())
            .unwrap_or("")
            .to_string()
    };
    let copyright = assertions.read()[idx].data["copyrightNotice"]
        .as_str().unwrap_or("").to_string();

    rsx! {
        div { class: "assertion-editor",
            div { class: "field",
                label { "Author" }
                input {
                    r#type: "text",
                    value: "{author}",
                    placeholder: "Name",
                    oninput: move |e| {
                        let mut a = assertions.write();
                        a[idx].data["author"] = json!([{ "@type": "Person", "name": e.value() }]);
                    }
                }
            }
            div { class: "field",
                label { "Copyright Notice" }
                input {
                    r#type: "text",
                    value: "{copyright}",
                    placeholder: "© 2024 Author",
                    oninput: move |e| {
                        assertions.write()[idx].data["copyrightNotice"] = json!(e.value());
                    }
                }
            }
        }
    }
}

/// Fallback editor: shows the raw JSON in a textarea.
#[component]
fn JsonEditor(assertions: Signal<Vec<AssertionEntry>>, idx: usize) -> Element {
    let json_text = {
        let a = assertions.read();
        serde_json::to_string_pretty(&a[idx].data).unwrap_or_default()
    };

    rsx! {
        div { class: "assertion-editor",
            div { class: "field",
                label { "Data (JSON)" }
                textarea {
                    class: "json-textarea",
                    value: "{json_text}",
                    rows: "4",
                    oninput: move |e| {
                        if let Ok(v) = serde_json::from_str::<Value>(&e.value()) {
                            assertions.write()[idx].data = v;
                        }
                    }
                }
            }
        }
    }
}

// ── main page ─────────────────────────────────────────────────────────────────

#[component]
pub fn SignPage() -> Element {
    let mut file: Signal<Option<String>> = use_signal(|| None);
    // dest for "Sign Asset" (signed copy of the file)
    let mut signed_dest: Signal<String> = use_signal(String::new);
    // dest for "Add Manifest" (.c2pa archive)
    let mut manifest_dest: Signal<String> = use_signal(String::new);
    let mut title: Signal<String> = use_signal(String::new);
    let mut assertions: Signal<Vec<AssertionEntry>> = use_signal(Vec::new);
    let mut ingredients: Signal<Vec<IngredientEntry>> = use_signal(Vec::new);
    // Load saved signer preferences once on first render.
    let saved_prefs = use_hook(load_preferences);
    let mut cert: Signal<String> = use_signal(|| saved_prefs.cert_path.clone());
    let mut key: Signal<String> = use_signal(|| saved_prefs.key_path.clone());
    let mut alg: Signal<SigningAlg> = use_signal(|| {
        // Convert stored string back to SigningAlg enum.
        ALGS.iter()
            .find(|a| format!("{a:?}") == saved_prefs.alg)
            .copied()
            .unwrap_or(SigningAlg::Es256)
    });
    let mut sign_result: Signal<SignResult> = use_signal(|| SignResult::Idle);
    let mut busy: Signal<bool> = use_signal(|| false);
    let mut custom_label: Signal<String> = use_signal(String::new);

    let persist_prefs = move || {
        let prefs = Preferences {
            cert_path: cert.read().clone(),
            key_path: key.read().clone(),
            alg: format!("{:?}", *alg.read()),
        };
        save_preferences(&prefs);
    };

    let has_file = move || file.read().is_some();
    let can_add_manifest = move || has_file() && !manifest_dest.read().is_empty();
    let can_sign = move || {
        has_file()
            && !signed_dest.read().is_empty()
            && !cert.read().is_empty()
            && !key.read().is_empty()
    };

    let mut add_assertion = move |label: String| {
        if label.is_empty() { return; }
        if assertions.read().iter().any(|a| a.label == label) { return; }
        assertions.write().push(AssertionEntry {
            data: default_data_for(&label),
            label,
        });
    };

    rsx! {
        // ── Fixed toast notification ──────────────────────────────────────────
        match sign_result.read().clone() {
            SignResult::Success(msg) => rsx! {
                div { class: "toast toast-success",
                    div { class: "toast-body",
                        span { class: "toast-icon", "✓" }
                        div {
                            div { class: "toast-title", "Done" }
                            div { class: "toast-path", "{msg}" }
                        }
                    }
                    button {
                        class: "toast-dismiss",
                        onclick: move |_| sign_result.set(SignResult::Idle),
                        "✕"
                    }
                }
            },
            SignResult::Error(msg) => rsx! {
                div { class: "toast toast-error",
                    div { class: "toast-body",
                        span { class: "toast-icon", "✕" }
                        div {
                            div { class: "toast-title", "Failed" }
                            div { class: "toast-path", "{msg}" }
                        }
                    }
                    button {
                        class: "toast-dismiss",
                        onclick: move |_| sign_result.set(SignResult::Idle),
                        "✕"
                    }
                }
            },
            SignResult::Idle => rsx! {},
        }

        div { class: "page-title", "Sign Asset" }
        div { class: "two-panel",

            // ── Left panel ────────────────────────────────────────────────────
            div { class: "panel-left",

                div { class: "drop-zone",
                    p { "Drop file here or" }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| {
                            info!(target: "c2pa_sample_app::ui::sign", "Browse dialog opened for source asset");
                            spawn(async move {
                                if let Some(h) = rfd::AsyncFileDialog::new()
                                    .add_filter("Assets", &["jpg","jpeg","png","mp4","mov","pdf","tiff","webp"])
                                    .add_filter("All files", &["*"])
                                    .pick_file().await
                                {
                                    let path = h.path().to_string_lossy().to_string();
                                    debug!(target: "c2pa_sample_app::ui::sign", "Source asset selected: {path}");
                                    signed_dest.set(derive_signed_dest(&path));
                                    manifest_dest.set(derive_manifest_dest(&path));
                                    file.set(Some(path));
                                    sign_result.set(SignResult::Idle);
                                } else {
                                    debug!(target: "c2pa_sample_app::ui::sign", "Browse dialog cancelled");
                                }
                            });
                        },
                        "Browse"
                    }
                }

                if let Some(f) = file.read().clone() {
                    div { class: "file-selected", "✓ {f}" }
                }

                // Signer
                div { class: "card",
                    div { class: "card-title", "Signer" }
                    div { class: "field",
                        label { "Algorithm" }
                        select {
                            class: "field-select",
                            onchange: move |e| {
                                let s = e.value();
                                if let Some(a) = ALGS.iter().find(|a| format!("{a:?}") == s) {
                                    alg.set(*a);
                                    persist_prefs();
                                }
                            },
                            for a in ALGS {
                                option { value: "{a:?}", selected: *alg.read() == *a, "{alg_label(a)}" }
                            }
                        }
                    }
                    div { class: "field",
                        label { "Certificate (.pem)" }
                        div { class: "inline-row",
                            input {
                                r#type: "text", value: "{cert}", placeholder: "cert.pem", style: "flex:1",
                                oninput: move |e| { cert.set(e.value()); persist_prefs(); }
                            }
                            button {
                                class: "btn btn-sm",
                                onclick: move |_| { spawn(async move {
                                    if let Some(h) = rfd::AsyncFileDialog::new()
                                        .add_filter("PEM", &["pem","crt"])
                                        .add_filter("All files", &["*"])
                                        .pick_file().await
                                    { cert.set(h.path().to_string_lossy().to_string()); persist_prefs(); }
                                }); },
                                "Browse"
                            }
                        }
                    }
                    div { class: "field",
                        label { "Private Key (.pem)" }
                        div { class: "inline-row",
                            input {
                                r#type: "text", value: "{key}", placeholder: "key.pem", style: "flex:1",
                                oninput: move |e| { key.set(e.value()); persist_prefs(); }
                            }
                            button {
                                class: "btn btn-sm",
                                onclick: move |_| { spawn(async move {
                                    if let Some(h) = rfd::AsyncFileDialog::new()
                                        .add_filter("PEM", &["pem","key"])
                                        .add_filter("All files", &["*"])
                                        .pick_file().await
                                    { key.set(h.path().to_string_lossy().to_string()); persist_prefs(); }
                                }); },
                                "Browse"
                            }
                        }
                    }
                }
            }

            // ── Right panel ───────────────────────────────────────────────────
            div { class: "panel-right",

                // Manifest
                div { class: "card",
                    div { class: "card-title", "Manifest" }
                    div { class: "field",
                        label { "Title" }
                        input {
                            r#type: "text", value: "{title}",
                            placeholder: "Leave blank to use filename",
                            oninput: move |e| title.set(e.value())
                        }
                    }
                    // Output for "Add Manifest"
                    div { class: "field",
                        label { "Manifest Archive (.c2pa)" }
                        div { class: "inline-row",
                            input {
                                r#type: "text", value: "{manifest_dest}",
                                placeholder: "Derived automatically",
                                style: "flex:1",
                                oninput: move |e| manifest_dest.set(e.value())
                            }
                            button {
                                class: "btn btn-sm",
                                onclick: move |_| {
                                    spawn(async move {
                                        if let Some(h) = rfd::AsyncFileDialog::new()
                                            .add_filter("C2PA Archive", &["c2pa"])
                                            .save_file().await
                                        { manifest_dest.set(h.path().to_string_lossy().to_string()); }
                                    });
                                },
                                "Browse"
                            }
                        }
                    }
                    // Output for "Sign Asset"
                    div { class: "field",
                        label { "Signed Output File" }
                        div { class: "inline-row",
                            input {
                                r#type: "text", value: "{signed_dest}",
                                placeholder: "Derived automatically",
                                style: "flex:1",
                                oninput: move |e| signed_dest.set(e.value())
                            }
                            button {
                                class: "btn btn-sm",
                                onclick: move |_| {
                                    let src = file.read().clone().unwrap_or_default();
                                    spawn(async move {
                                        let ext = std::path::Path::new(&src)
                                            .extension().and_then(|e| e.to_str())
                                            .unwrap_or("bin").to_string();
                                        if let Some(h) = rfd::AsyncFileDialog::new()
                                            .add_filter("Same type", &[&ext])
                                            .save_file().await
                                        { signed_dest.set(h.path().to_string_lossy().to_string()); }
                                    });
                                },
                                "Browse"
                            }
                        }
                    }
                }

                // Assertions
                div { class: "card",
                    div { class: "card-title", "Assertions" }

                    // List
                    for (idx, entry) in assertions.read().iter().enumerate() {
                        {
                            let label = entry.label.clone();
                            let label_disp = label.clone();
                            rsx! {
                                div { class: "assertion-item",
                                    // Header row
                                    div { class: "assertion-header",
                                        span { class: "assertion-label", "{label_disp}" }
                                        button {
                                            class: "btn btn-sm btn-danger",
                                            onclick: move |_| { assertions.write().remove(idx); },
                                            "✕"
                                        }
                                    }
                                    // Inline editor based on label
                                    match label.as_str() {
                                        "c2pa.actions" => rsx! {
                                            ActionsEditor { assertions, idx }
                                        },
                                        "c2pa.training-mining" => rsx! {
                                            TrainingMiningEditor { assertions, idx }
                                        },
                                        "stds.schema-org.CreativeWork" => rsx! {
                                            CreativeWorkEditor { assertions, idx }
                                        },
                                        _ => rsx! {
                                            JsonEditor { assertions, idx }
                                        },
                                    }
                                }
                            }
                        }
                    }

                    if assertions.read().is_empty() {
                        p { class: "empty-state", "No assertions — a bare manifest will be signed" }
                    }

                    // Add row: preset dropdown + custom label input
                    div { class: "assertion-add-row",
                        select {
                            class: "field-select",
                            onchange: move |e| {
                                let v = e.value();
                                if !v.is_empty() { add_assertion(v); }
                            },
                            option { value: "", "— preset —" }
                            for label in PRESET_ASSERTIONS {
                                option { value: "{label}", "{label}" }
                            }
                        }
                        input {
                            r#type: "text",
                            class: "custom-assertion-input",
                            value: "{custom_label}",
                            placeholder: "custom label…",
                            oninput: move |e| custom_label.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter {
                                    let v = custom_label.read().trim().to_string();
                                    if !v.is_empty() {
                                        add_assertion(v);
                                        custom_label.set(String::new());
                                    }
                                }
                            }
                        }
                        button {
                            class: "btn btn-sm",
                            onclick: move |_| {
                                let v = custom_label.read().trim().to_string();
                                if !v.is_empty() {
                                    add_assertion(v);
                                    custom_label.set(String::new());
                                }
                            },
                            "Add"
                        }
                    }
                }

                // Ingredients
                div { class: "card",
                    div { class: "card-title", "Ingredients" }

                    for (idx, ing) in ingredients.read().iter().enumerate() {
                        {
                            let name = std::path::Path::new(&ing.path)
                                .file_name().and_then(|n| n.to_str())
                                .unwrap_or(&ing.path).to_string();
                            let rel = ing.relationship.clone();
                            rsx! {
                                div { class: "ingredient-item",
                                    div { class: "ingredient-header",
                                        span { class: "ingredient-name", "{name}" }
                                        select {
                                            class: "field-select ingredient-rel",
                                            onchange: move |e| {
                                                ingredients.write()[idx].relationship = e.value();
                                            },
                                            for (val, lbl) in RELATIONSHIPS {
                                                option {
                                                    value: "{val}",
                                                    selected: *val == rel.as_str(),
                                                    "{lbl}"
                                                }
                                            }
                                        }
                                        button {
                                            class: "btn btn-sm btn-danger",
                                            onclick: move |_| { ingredients.write().remove(idx); },
                                            "✕"
                                        }
                                    }
                                    div { class: "ingredient-path", "{ing.path}" }
                                }
                            }
                        }
                    }

                    if ingredients.read().is_empty() {
                        p { class: "empty-state", "No ingredients" }
                    }

                    div { class: "add-row",
                        button {
                            class: "btn btn-sm",
                            onclick: move |_| {
                                spawn(async move {
                                    if let Some(h) = rfd::AsyncFileDialog::new()
                                        .add_filter("All files", &["*"])
                                        .pick_file().await
                                    {
                                        ingredients.write().push(IngredientEntry {
                                            path: h.path().to_string_lossy().to_string(),
                                            relationship: "componentOf".to_string(),
                                            title: None,
                                        });
                                    }
                                });
                            },
                            "+ Add Ingredient"
                        }
                    }
                }

                // ── Action buttons ────────────────────────────────────────────
                div { class: "action-buttons",

                    // Add Manifest — no cert/key required
                    button {
                        class: "btn btn-full",
                        disabled: !can_add_manifest() || *busy.read(),
                        title: "Export an unsigned .c2pa manifest archive",
                        onclick: move |_| {
                            let manifest_p = ManifestParams {
                                source:     file.read().clone().unwrap_or_default(),
                                title:      { let t = title.read().clone(); if t.is_empty() { None } else { Some(t) } },
                                format:     None,
                                assertions: assertions.read().iter().map(|a| (a.label.clone(), a.data.clone())).collect(),
                                ingredients: ingredients.read().clone(),
                            };
                            let out = manifest_dest.read().clone();
                            busy.set(true);
                            sign_result.set(SignResult::Idle);
                            spawn(async move {
                                let result = add_manifest(manifest_p, out);
                                busy.set(false);
                                match result {
                                    Ok(path) => sign_result.set(SignResult::Success(
                                        format!("Manifest archive written to {path}")
                                    )),
                                    Err(e) => sign_result.set(SignResult::Error(e)),
                                }
                            });
                        },
                        if *busy.read() { "Working…" } else { "Add Manifest" }
                    }

                    // Sign Asset — requires cert + key
                    button {
                        class: "btn btn-primary btn-full",
                        disabled: !can_sign() || *busy.read(),
                        title: "Sign the asset and embed the manifest",
                        onclick: move |_| {
                            let params = SignParams {
                                manifest: ManifestParams {
                                    source:     file.read().clone().unwrap_or_default(),
                                    title:      { let t = title.read().clone(); if t.is_empty() { None } else { Some(t) } },
                                    format:     None,
                                    assertions: assertions.read().iter().map(|a| (a.label.clone(), a.data.clone())).collect(),
                                    ingredients: ingredients.read().clone(),
                                },
                                dest:      signed_dest.read().clone(),
                                cert_path: cert.read().clone(),
                                key_path:  key.read().clone(),
                                alg:       *alg.read(),
                            };
                            busy.set(true);
                            sign_result.set(SignResult::Idle);
                            spawn(async move {
                                let result = sign_asset(params);
                                busy.set(false);
                                match result {
                                    Ok(path) => sign_result.set(SignResult::Success(
                                        format!("Signed asset written to {path}")
                                    )),
                                    Err(e) => sign_result.set(SignResult::Error(e)),
                                }
                            });
                        },
                        if *busy.read() { "Working…" } else { "Sign Asset" }
                    }
                }
            }
        }
    }
}
