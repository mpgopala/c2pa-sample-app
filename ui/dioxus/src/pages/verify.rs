use c2pa_sample_app::model::manifest::{
    verify_embedded_manifest, ManifestSummary, VerifyResult, VerifyValidationState,
};
use c2pa_sample_app::model::recents::{push_recent, RecentEntry};
use crate::menu::rebuild_recents_menu;
use dioxus::prelude::*;
use serde_json::Value;
use std::collections::HashSet;
use tracing::{debug, info};

// ── flat tree data model ─────────────────────────────────────────────────────

/// A single row in the tree.  Every row has a unique hierarchical `id`
/// (slash-separated path, e.g. "root/active/assertions/c2pa.actions").
/// Leaves carry a `value`; sections are collapsible.
/// `ingredient_link` is set on leaves whose value is a JUMBF ingredient
/// reference — it holds the ingredient assertion label to navigate to.
#[derive(Clone, PartialEq)]
struct Row {
    id: String,
    label: String,
    depth: usize,
    is_section: bool,
    value: Option<String>,
    ingredient_link: Option<String>,
}

/// IDs of all section rows (needed to decide visibility).
type SectionSet = HashSet<String>;

/// Extract the ingredient assertion label from a JUMBF ingredient URI.
/// e.g. "self#jumbf=c2pa.assertions/c2pa.ingredient.v3" → "c2pa.ingredient.v3"
fn extract_ingredient_label(value: &str) -> Option<&str> {
    let prefix = "self#jumbf=c2pa.assertions/";
    value
        .strip_prefix(prefix)
        .filter(|s| s.starts_with("c2pa.ingredient"))
}

/// Return the tree row ID for the ingredient whose assertion label matches
/// `label`, searching the active manifest first then the others.
fn find_ingredient_row_id(result: &VerifyResult, label: &str) -> Option<String> {
    if let Some(m) = &result.manifest {
        for ing in &m.ingredients {
            if ing.label.as_deref() == Some(label) {
                return Some(format!("root/active/ingredients/{}", ing.instance_id));
            }
        }
    }
    for (idx, m) in result.all_manifests.iter().enumerate() {
        for ing in &m.ingredients {
            if ing.label.as_deref() == Some(label) {
                return Some(format!("root/other/{idx}/ingredients/{}", ing.instance_id));
            }
        }
    }
    None
}

/// Return all proper ancestor IDs for a slash-separated row ID.
/// e.g. "root/active/ingredients/foo" → ["root", "root/active", "root/active/ingredients"]
fn ancestor_ids(row_id: &str) -> Vec<String> {
    let parts: Vec<&str> = row_id.split('/').collect();
    (1..parts.len()).map(|i| parts[..i].join("/")).collect()
}


fn push_leaf(rows: &mut Vec<Row>, id: impl Into<String>, label: impl Into<String>, depth: usize, value: impl Into<String>) {
    let value = value.into();
    let ingredient_link = extract_ingredient_label(&value).map(str::to_string);
    rows.push(Row { id: id.into(), label: label.into(), depth, is_section: false, value: Some(value), ingredient_link });
}

fn push_section(rows: &mut Vec<Row>, sections: &mut SectionSet, id: impl Into<String>, label: impl Into<String>, depth: usize) {
    let id = id.into();
    sections.insert(id.clone());
    rows.push(Row { id, label: label.into(), depth, is_section: true, value: None, ingredient_link: None });
}

/// Recursively adds rows for a JSON value.
/// Objects and arrays become expandable sections; scalars become leaves.
fn build_json_rows(
    rows: &mut Vec<Row>,
    sections: &mut SectionSet,
    v: &Value,
    id: &str,
    label: &str,
    depth: usize,
) {
    match v {
        Value::Object(map) => {
            push_section(rows, sections, id, format!("{label} {{…}}"), depth);
            for (k, child) in map {
                build_json_rows(rows, sections, child, &format!("{id}/{k}"), k, depth + 1);
            }
        }
        Value::Array(arr) => {
            push_section(rows, sections, id, format!("{label} [{}]", arr.len()), depth);
            for (i, child) in arr.iter().enumerate() {
                build_json_rows(rows, sections, child, &format!("{id}/{i}"), &format!("[{i}]"), depth + 1);
            }
        }
        Value::String(s)  => push_leaf(rows, id, label, depth, s.as_str()),
        Value::Null       => push_leaf(rows, id, label, depth, "null"),
        Value::Bool(b)    => push_leaf(rows, id, label, depth, b.to_string()),
        Value::Number(n)  => push_leaf(rows, id, label, depth, n.to_string()),
    }
}

fn build_manifest_rows(
    rows: &mut Vec<Row>,
    sections: &mut SectionSet,
    m: &ManifestSummary,
    prefix: &str,
    depth: usize,
) {
    if let Some(t) = &m.title {
        push_leaf(rows, format!("{prefix}/title"), "title", depth, t);
    }
    if let Some(f) = &m.format {
        push_leaf(rows, format!("{prefix}/format"), "format", depth, f);
    }
    if let Some(cg) = &m.claim_generator {
        push_leaf(rows, format!("{prefix}/claim_generator"), "claim_generator", depth, cg);
    }
    push_leaf(rows, format!("{prefix}/instance_id"), "instance_id", depth, &m.instance_id);

    if m.issuer.is_some() || m.signing_time.is_some() {
        let sig = format!("{prefix}/signature");
        push_section(rows, sections, &sig, "signature", depth);
        if let Some(iss) = &m.issuer {
            push_leaf(rows, format!("{sig}/issuer"), "issuer", depth + 1, iss);
        }
        if let Some(ts) = &m.signing_time {
            push_leaf(rows, format!("{sig}/time"), "time", depth + 1, ts);
        }
    }

    let asn_id = format!("{prefix}/assertions");
    push_section(rows, sections, &asn_id, format!("assertions ({})", m.assertions.len()), depth);
    for a in &m.assertions {
        let a_id = format!("{asn_id}/{}", a.label);
        match &a.data {
            Value::Object(map) => {
                push_section(rows, sections, &a_id, a.label.clone(), depth + 1);
                for (k, v) in map {
                    build_json_rows(rows, sections, v, &format!("{a_id}/{k}"), k, depth + 2);
                }
            }
            other => {
                build_json_rows(rows, sections, other, &a_id, &a.label, depth + 1);
            }
        }
    }

    let ing_id = format!("{prefix}/ingredients");
    push_section(rows, sections, &ing_id, format!("ingredients ({})", m.ingredients.len()), depth);
    for ing in &m.ingredients {
        let title = ing.title.clone().unwrap_or_else(|| "(untitled)".to_string());
        let i_id = format!("{ing_id}/{}", ing.instance_id);
        push_section(rows, sections, &i_id, format!("{title} [{}]", ing.relationship), depth + 1);
        if let Value::Object(map) = &ing.data {
            for (k, v) in map {
                build_json_rows(rows, sections, v, &format!("{i_id}/{k}"), k, depth + 2);
            }
        }
    }
}

fn build_full_tree(result: &VerifyResult) -> (Vec<Row>, SectionSet) {
    let mut rows: Vec<Row> = Vec::new();
    let mut sections: SectionSet = HashSet::new();

    push_section(&mut rows, &mut sections, "root", "ManifestStore", 0);

    if let Some(m) = &result.manifest {
        let active_id = "root/active";
        push_section(&mut rows, &mut sections, active_id, format!("{} (active)", m.label), 1);
        build_manifest_rows(&mut rows, &mut sections, m, active_id, 2);
    }

    for (idx, m) in result.all_manifests.iter().enumerate() {
        let node_id = format!("root/other/{idx}");
        push_section(&mut rows, &mut sections, &node_id, m.label.clone(), 1);
        build_manifest_rows(&mut rows, &mut sections, m, &node_id, 2);
    }

    (rows, sections)
}

fn is_visible(id: &str, sections: &SectionSet, expanded: &HashSet<String>) -> bool {
    let parts: Vec<&str> = id.split('/').collect();
    for len in 1..parts.len() {
        let ancestor = parts[..len].join("/");
        if sections.contains(&ancestor) && !expanded.contains(&ancestor) {
            return false;
        }
    }
    true
}

// ── shared helper: open a file path for verification ────────────────────────

fn default_expanded() -> HashSet<String> {
    ["root", "root/active", "valroot"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

// ── page component ───────────────────────────────────────────────────────────

#[component]
pub fn VerifyPage() -> Element {
    let mut file: Signal<Option<String>> = use_signal(|| None);
    let mut result: Signal<Option<VerifyResult>> = use_signal(|| None);
    let mut expanded: Signal<HashSet<String>> = use_signal(default_expanded);
    let mut highlighted: Signal<Option<String>> = use_signal(|| None);
    let mut recents: Signal<Vec<RecentEntry>> = use_context();
    let mut pending_open: Signal<Option<String>> = use_context();

    // Verify a file path and update all relevant state.
    // Also rebuilds the native "Recent Files" menu after updating recents.
    let mut open_file = move |path: String| {
        info!(target: "c2pa_tool::ui::verify", "Opening file for verification: {path}");
        file.set(Some(path.clone()));
        let verify_result = verify_embedded_manifest(&path);
        debug!(target: "c2pa_tool::ui::verify", "Verification complete, state: {:?}", verify_result.state);
        push_recent(&path, &mut recents.write());
        rebuild_recents_menu(&recents.peek());
        expanded.set(default_expanded());
        highlighted.set(None);
        result.set(Some(verify_result));
    };

    // Respond to files queued by the native menu (File > Open… or File > Recent Files).
    // Use spawn to break out of the reactive scope before mutating pending_open,
    // preventing Dioxus from detecting a read-then-write cycle on the same signal.
    use_effect(move || {
        let queued = pending_open.read().clone();
        if let Some(path) = queued {
            spawn(async move {
                pending_open.set(None);
                open_file(path);
            });
        }
    });

    rsx! {
        div { class: "page-title", "Verify Asset" }
        div { class: "two-panel",
            // ── Left panel ──────────────────────────────────────────────────
            div { class: "panel-left",
                div { class: "drop-zone",
                    p { "Drop file here or" }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| {
                            info!(target: "c2pa_tool::ui::verify", "Browse dialog opened");
                            spawn(async move {
                                if let Some(handle) = rfd::AsyncFileDialog::new().pick_file().await {
                                    let path_str = handle.path().to_string_lossy().to_string();
                                    open_file(path_str);
                                } else {
                                    debug!(target: "c2pa_tool::ui::verify", "Browse dialog cancelled");
                                }
                            });
                        },
                        "Browse"
                    }
                }

                if let Some(f) = file.read().clone() {
                    div { class: "file-selected", "✓ {f}" }
                }


                // ── Thumbnail + Validation summary ────────────────────────
                if let Some(uri) = result.read().as_ref().and_then(|r| r.manifest.as_ref()).and_then(|m| m.thumbnail_data_uri.clone()) {
                    div { class: "card",
                        div { class: "card-title", "Thumbnail" }
                        img {
                            src: "{uri}",
                            style: "max-width: 100%; border-radius: 4px;",
                            alt: "Asset thumbnail"
                        }
                    }
                }
                if let Some(res) = result.read().as_ref() {
                    {
                        let (badge_class, state_label) = match res.state {
                            VerifyValidationState::Trusted    => ("status-badge status-verified", "TRUSTED"),
                            VerifyValidationState::Valid      => ("status-badge status-verified", "VALID"),
                            VerifyValidationState::Invalid    => ("status-badge status-tampered", "INVALID"),
                            VerifyValidationState::NoManifest => ("status-badge status-unsigned", "NO MANIFEST"),
                        };
                        let issuer   = res.manifest.as_ref().and_then(|m| m.issuer.clone());
                        let sig_time = res.manifest.as_ref().and_then(|m| m.signing_time.clone());
                        rsx! {
                            div { class: "card",
                                div { class: "card-title", "Validation" }
                                div { class: "{badge_class}",
                                    span { class: "status-dot" }
                                    "{state_label}"
                                }
                                if let Some(iss) = issuer {
                                    div { class: "meta-row",
                                        span { class: "meta-label", "Signed by" }
                                        span { "{iss}" }
                                    }
                                }
                                if let Some(ts) = sig_time {
                                    div { class: "meta-row",
                                        span { class: "meta-label", "Timestamp" }
                                        span { "{ts}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ── Right panel ─────────────────────────────────────────────────
            div { class: "panel-right",
                div { class: "card",
                    div { class: "card-title", "Manifest Store" }
                    if result.read().is_none() {
                        p { style: "color: var(--text-muted); font-style: italic;",
                            "Select a file to inspect"
                        }
                    } else if result.read().as_ref().map(|r| r.manifest.is_none()).unwrap_or(false) {
                        p { style: "color: var(--text-muted); font-style: italic;",
                            "No C2PA manifest found in this file."
                        }
                    } else {
                        {
                            let res_read = result.read();
                            let res = res_read.as_ref().unwrap();
                            let (all_rows, sections) = build_full_tree(res);
                            let exp = expanded.read();

                            let visible: Vec<Row> = all_rows
                                .iter()
                                .filter(|r| is_visible(&r.id, &sections, &exp))
                                .cloned()
                                .collect();
                            drop(exp);

                            rsx! {
                                div { class: "tree",
                                    for row in visible {
                                        {
                                            let indent = row.depth * 16;
                                            let row_id = row.id.clone();
                                            let is_highlighted = highlighted.read().as_deref() == Some(&row_id);
                                            if row.is_section {
                                                let is_open = expanded.read().contains(&row_id);
                                                let section_class = if is_highlighted {
                                                    "tree-node tree-section tree-highlighted"
                                                } else {
                                                    "tree-node tree-section"
                                                };
                                                rsx! {
                                                    div {
                                                        key: "{row_id}",
                                                        class: "{section_class}",
                                                        style: "padding-left: {indent}px",
                                                        onclick: move |_| {
                                                            highlighted.set(None);
                                                            let mut exp = expanded.write();
                                                            if exp.contains(&row_id) { exp.remove(&row_id); } else { exp.insert(row_id.clone()); }
                                                        },
                                                        span { class: "tree-icon", if is_open { "▾" } else { "▸" } }
                                                        "{row.label}"
                                                    }
                                                }
                                            } else {
                                                let value = row.value.clone().unwrap_or_default();
                                                let ing_link = row.ingredient_link.clone();
                                                rsx! {
                                                    div {
                                                        key: "{row_id}",
                                                        class: "tree-leaf",
                                                        style: "padding-left: {indent}px",
                                                        span { class: "tree-key", "{row.label}" }
                                                        span { class: "tree-sep", ": " }
                                                        span { class: "tree-value", "{value}" }
                                                        if let Some(ing_label) = ing_link {
                                                            span {
                                                                class: "tree-ing-link",
                                                                onclick: move |e| {
                                                                    e.stop_propagation();
                                                                    let res_guard = result.read();
                                                                    if let Some(res) = res_guard.as_ref() {
                                                                        if let Some(target_id) = find_ingredient_row_id(res, &ing_label) {
                                                                            let mut exp = expanded.write();
                                                                            for anc in ancestor_ids(&target_id) {
                                                                                exp.insert(anc);
                                                                            }
                                                                            exp.insert(target_id.clone());
                                                                            drop(exp);
                                                                            highlighted.set(Some(target_id));
                                                                        }
                                                                    }
                                                                },
                                                                "↗ ingredient"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                div { class: "add-row",
                                    button { class: "btn", "Export Report" }
                                }
                            }
                        }
                    }
                }
                if let Some(res_read) = result.read().as_ref() {
                    {
                        let val_json = serde_json::Value::Array(res_read.validation_statuses.clone());
                        let mut val_rows: Vec<Row> = Vec::new();
                        let mut val_sections: SectionSet = HashSet::new();
                        build_json_rows(&mut val_rows, &mut val_sections, &val_json, "valroot", format!("ValidationStatuses ({})", res_read.validation_statuses.len()).as_str(), 0);
                        let exp = expanded.read();
                        let visible: Vec<Row> = val_rows
                            .iter()
                            .filter(|r| is_visible(&r.id, &val_sections, &exp))
                            .cloned()
                            .collect();
                        drop(exp);
                        rsx! {
                            div { class: "card",
                                div { class: "card-title", "Validation Results" }
                                if visible.is_empty() {
                                    p { style: "color: var(--text-muted); font-style: italic;",
                                        "No validation statuses reported."
                                    }
                                } else {
                                    div { class: "tree",
                                        for row in visible {
                                            {
                                                let indent = row.depth * 16;
                                                let row_id = row.id.clone();
                                                if row.is_section {
                                                    let is_open = expanded.read().contains(&row_id);
                                                    rsx! {
                                                        div {
                                                            key: "{row_id}",
                                                            class: "tree-node tree-section",
                                                            style: "padding-left: {indent}px",
                                                            onclick: move |_| {
                                                                let mut exp = expanded.write();
                                                                if exp.contains(&row_id) { exp.remove(&row_id); } else { exp.insert(row_id.clone()); }
                                                            },
                                                            span { class: "tree-icon", if is_open { "▾" } else { "▸" } }
                                                            "{row.label}"
                                                        }
                                                    }
                                                } else {
                                                    let value = row.value.clone().unwrap_or_default();
                                                    rsx! {
                                                        div {
                                                            key: "{row_id}",
                                                            class: "tree-leaf",
                                                            style: "padding-left: {indent}px",
                                                            span { class: "tree-key", "{row.label}" }
                                                            span { class: "tree-sep", ": " }
                                                            span { class: "tree-value", "{value}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
