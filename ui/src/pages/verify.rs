use c2pa::assertions::labels;
use model::manifest::{
    verify_embedded_manifest, IngredientSummary, ManifestSummary, VerifyResult,
    VerifyValidationState,
};
use model::recents::{push_recent, RecentEntry};
use dioxus::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

// ── manifest store relationship diagram (recursive tree) ───────────────────

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ManifestNodeSeverity {
    Ok,
    Warn,
    Error,
}

fn manifest_label_map(result: &VerifyResult) -> HashMap<String, ManifestSummary> {
    let mut map = HashMap::new();
    if let Some(m) = &result.manifest {
        map.insert(m.label.clone(), m.clone());
    }
    for m in &result.all_manifests {
        map.insert(m.label.clone(), m.clone());
    }
    map
}

fn relationship_sort_key(rel: &str) -> u8 {
    match rel {
        "parentOf" => 0,
        "componentOf" => 1,
        "inputTo" => 2,
        _ => 3,
    }
}

/// Tree section id in the Manifest Store panel for a manifest (`root/active` or `root/other/{i}`).
fn manifest_tree_section_id(label: &str, result: &VerifyResult) -> Option<String> {
    if result.manifest.as_ref().is_some_and(|m| m.label == label) {
        return Some("root/active".to_string());
    }
    for (idx, m) in result.all_manifests.iter().enumerate() {
        if m.label == label {
            return Some(format!("root/other/{idx}"));
        }
    }
    None
}

/// Recursive manifest tree: ingredients whose `active_manifest` resolves to another claim
/// in this asset become embedded child nodes. Ingredients **without** a resolvable embedded
/// manifest (e.g. `componentOf` file-only references like I.jpg) still appear as
/// **ingredient-only** stubs. **`parentOf` ingredients are omitted** (provenance parent, not a child).
#[derive(Clone, PartialEq)]
struct ManifestRelNode {
    summary: ManifestSummary,
    children: Vec<ManifestRelChild>,
}

#[derive(Clone, PartialEq)]
enum ManifestRelChild {
    Embedded(ManifestRelNode),
    IngredientOnly(IngredientSummary),
}

fn build_manifest_rel_tree(
    m: &ManifestSummary,
    map: &HashMap<String, ManifestSummary>,
    path: &mut HashSet<String>,
) -> ManifestRelNode {
    path.insert(m.label.clone());
    let mut items: Vec<(u8, String, ManifestRelChild)> = Vec::new();

    for ing in &m.ingredients {
        if ing.relationship == "parentOf" {
            continue;
        }
        let rk = relationship_sort_key(&ing.relationship);
        if let Some(l) = ing.active_manifest.as_ref() {
            if !path.contains(l) {
                if let Some(child_sum) = map.get(l) {
                    let sub = build_manifest_rel_tree(child_sum, map, path);
                    items.push((
                        rk,
                        l.clone(),
                        ManifestRelChild::Embedded(sub),
                    ));
                    continue;
                }
            }
        }
        // No embedded manifest in this asset’s store (or unresolved label): show ingredient stub.
        items.push((
            rk,
            ing.instance_id.clone(),
            ManifestRelChild::IngredientOnly(ing.clone()),
        ));
    }

    items.sort_by(|a, b| {
        a.0.cmp(&b.0).then_with(|| match (&a.2, &b.2) {
            (ManifestRelChild::Embedded(x), ManifestRelChild::Embedded(y)) => {
                x.summary.label.cmp(&y.summary.label)
            }
            (ManifestRelChild::IngredientOnly(x), ManifestRelChild::IngredientOnly(y)) => {
                x.instance_id.cmp(&y.instance_id)
            }
            (ManifestRelChild::Embedded(_), ManifestRelChild::IngredientOnly(_)) => {
                std::cmp::Ordering::Less
            }
            (ManifestRelChild::IngredientOnly(_), ManifestRelChild::Embedded(_)) => {
                std::cmp::Ordering::Greater
            }
        })
    });

    let mut seen: HashSet<String> = HashSet::new();
    items.retain(|(_, key, _)| seen.insert(key.clone()));

    let children: Vec<ManifestRelChild> = items.into_iter().map(|(_, _, c)| c).collect();
    path.remove(&m.label);
    ManifestRelNode {
        summary: m.clone(),
        children,
    }
}

fn manifest_node_severity(m: &ManifestSummary, result: &VerifyResult, is_active: bool) -> ManifestNodeSeverity {
    if is_active {
        return match result.state {
            VerifyValidationState::Trusted => ManifestNodeSeverity::Ok,
            VerifyValidationState::Valid => ManifestNodeSeverity::Warn,
            VerifyValidationState::Invalid | VerifyValidationState::NoManifest => ManifestNodeSeverity::Error,
        };
    }
    let mut worst = ManifestNodeSeverity::Ok;
    let label = m.label.as_str();
    let iid = m.instance_id.as_str();
    for v in &result.validation_statuses {
        let blob = v.to_string();
        if !blob.contains(label) && !blob.contains(iid) {
            continue;
        }
        if let Some(code) = v.get("code").and_then(|c| c.as_str()) {
            if code.contains("failure")
                || (code.contains("invalid") && !code.contains("insideValidity"))
                || code.contains("mismatch")
            {
                worst = ManifestNodeSeverity::Error;
            } else if code.contains("untrusted")
                || code.contains("outsideValidity")
                || code.contains("ocsp")
            {
                worst = worst.max(ManifestNodeSeverity::Warn);
            }
        }
    }
    worst
}

#[component]
fn ManifestDiagramNode(
    title: String,
    subtitle: String,
    thumb: Option<String>,
    severity: ManifestNodeSeverity,
    is_active: bool,
    is_selected: bool,
) -> Element {
    let node_class = if is_selected {
        "mstore-node mstore-node-active"
    } else {
        "mstore-node mstore-node-secondary"
    };
    let badge_class = match severity {
        ManifestNodeSeverity::Ok => "mstore-badge mstore-badge-ok",
        ManifestNodeSeverity::Warn => "mstore-badge mstore-badge-warn",
        ManifestNodeSeverity::Error => "mstore-badge mstore-badge-error",
    };
    let icon = match severity {
        ManifestNodeSeverity::Ok => "✓",
        ManifestNodeSeverity::Warn => "!",
        ManifestNodeSeverity::Error => "!",
    };
    rsx! {
        div { class: "{node_class}",
            div { class: "{badge_class}", "{icon}" }
            if let Some(uri) = thumb {
                img { class: "mstore-thumb", src: "{uri}", alt: "" }
            } else {
                div { class: "mstore-thumb-placeholder", "No thumbnail" }
            }
            div { class: "mstore-title", "{title}" }
            div { class: "mstore-sub", "{subtitle}" }
        }
    }
}

#[component]
fn ManifestRelSubtree(
    node: ManifestRelNode,
    result: VerifyResult,
    active_label: String,
    expanded: Signal<HashSet<String>>,
    highlighted: Signal<Option<String>>,
) -> Element {
    let m = node.summary.clone();
    let label = m.label.clone();
    let title = m.title.clone().unwrap_or_else(|| m.label.clone());
    let subtitle = m.label.clone();
    let is_active_asset = label == active_label;
    let sev = manifest_node_severity(&m, &result, is_active_asset);
    let thumb = m.thumbnail_data_uri.clone();
    let tree_id = manifest_tree_section_id(&label, &result);
    let highlight_for_click = tree_id.clone();
    let active_tid = manifest_tree_section_id(&active_label, &result);

    rsx! {
        div { class: "mrel-subtree",
            div {
                class: "mrel-node-click",
                onclick: move |_| {
                    if let Some(ref id) = highlight_for_click {
                        let mut exp = expanded.write();
                        exp.insert("root".to_string());
                        for anc in ancestor_ids(id) {
                            exp.insert(anc);
                        }
                        exp.insert(id.clone());
                        highlighted.set(Some(id.clone()));
                    }
                },
                ManifestDiagramNode {
                    title: title,
                    subtitle: subtitle,
                    thumb: thumb,
                    severity: sev,
                    is_active: is_active_asset,
                    is_selected: diagram_manifest_is_selected(&tree_id, &active_tid, &highlighted.read().clone()),
                }
            }
            if !node.children.is_empty() {
                div { class: "mrel-connector-down" }
                div { class: "mrel-children-row",
                    for (ci, ch) in node.children.iter().enumerate() {
                        {
                            let parent_label = node.summary.label.clone();
                            match ch {
                                ManifestRelChild::Embedded(n) => {
                                    let n = n.clone();
                                    let clabel = n.summary.label.clone();
                                    rsx! {
                                        div { class: "mrel-child-column", key: "emb-{clabel}-{ci}",
                                            ManifestRelSubtree {
                                                node: n,
                                                result: result.clone(),
                                                active_label: active_label.clone(),
                                                expanded: expanded,
                                                highlighted: highlighted,
                                            }
                                        }
                                    }
                                }
                                ManifestRelChild::IngredientOnly(ing) => {
                                    let ing = ing.clone();
                                    let iid = ing.instance_id.clone();
                                    rsx! {
                                        div { class: "mrel-child-column", key: "ing-{iid}-{ci}",
                                            IngredientStubDiagram {
                                                ing: ing,
                                                parent_manifest_label: parent_label.clone(),
                                                result: result.clone(),
                                                active_label: active_label.clone(),
                                                expanded: expanded,
                                                highlighted: highlighted,
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

#[component]
fn ManifestRelationshipDiagram(
    result: VerifyResult,
    expanded: Signal<HashSet<String>>,
    highlighted: Signal<Option<String>>,
) -> Element {
    let Some(root) = result.manifest.clone() else {
        return rsx! {};
    };
    let map = manifest_label_map(&result);
    let mut path = HashSet::new();
    let tree = build_manifest_rel_tree(&root, &map, &mut path);
    let active_label = root.label.clone();

    rsx! {
        div { class: "card mrel-card",
            div { class: "card-title", "Manifest relationships" }
            div { class: "mrel-root",
                ManifestRelSubtree {
                    node: tree,
                    result: result.clone(),
                    active_label: active_label.clone(),
                    expanded: expanded,
                    highlighted: highlighted,
                }
            }
        }
    }
}

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

/// Extract the ingredient assertion path segment from a JUMBF URI (includes `__n` instance).
/// e.g. `.../c2pa.ingredient.v3__1` → `c2pa.ingredient.v3__1`
fn extract_ingredient_label(value: &str) -> Option<&str> {
    let prefix = "self#jumbf=c2pa.assertions/";
    value
        .strip_prefix(prefix)
        .filter(|s| s.starts_with("c2pa.ingredient"))
}

/// `root/active` or `root/other/{idx}` for any tree row under that manifest’s subtree.
fn manifest_prefix_from_tree_row_id(row_id: &str) -> Option<String> {
    if row_id == "root/active" || row_id.starts_with("root/active/") {
        return Some("root/active".to_string());
    }
    let parts: Vec<&str> = row_id.split('/').collect();
    if parts.len() >= 3 && parts[0] == "root" && parts[1] == "other" && parts[2].chars().all(|c| c.is_ascii_digit()) {
        return Some(format!("root/other/{}", parts[2]));
    }
    None
}

fn manifest_for_prefix<'a>(result: &'a VerifyResult, prefix: &str) -> Option<&'a ManifestSummary> {
    match prefix {
        "root/active" => result.manifest.as_ref(),
        p if p.starts_with("root/other/") => p
            .strip_prefix("root/other/")
            .and_then(|s| s.parse::<usize>().ok())
            .and_then(|i| result.all_manifests.get(i)),
        _ => None,
    }
}

/// True if `uri_suffix` (from JUMBF URI) and `stored` (ingredient assertion label) denote the same assertion.
fn ingredient_assertion_labels_match(uri_suffix: &str, stored: &str) -> bool {
    if uri_suffix.is_empty() || stored.is_empty() {
        return false;
    }
    if uri_suffix == stored {
        return true;
    }
    labels::base(uri_suffix) == labels::base(stored)
        && labels::version(uri_suffix) == labels::version(stored)
        && labels::instance(uri_suffix) == labels::instance(stored)
}

/// Resolve an ingredient JUMBF label to a row id **only within the manifest** that contains `source_row_id`.
fn find_ingredient_row_id(result: &VerifyResult, uri_suffix: &str, source_row_id: &str) -> Option<String> {
    let prefix = manifest_prefix_from_tree_row_id(source_row_id)?;
    let m = manifest_for_prefix(result, &prefix)?;
    for ing in &m.ingredients {
        let stored = ing.label.as_deref().unwrap_or("");
        if ingredient_assertion_labels_match(uri_suffix, stored) {
            return Some(format!("{prefix}/ingredients/{}", ing.instance_id));
        }
    }
    None
}

/// Diagram selection: default (`None`) highlights the active manifest only; any explicit selection replaces it.
fn diagram_manifest_is_selected(
    tree_id: &Option<String>,
    active_tree_id: &Option<String>,
    highlighted: &Option<String>,
) -> bool {
    match highlighted {
        None => tree_id == active_tree_id,
        Some(h) => tree_id.as_deref() == Some(h.as_str()),
    }
}

/// Tree row id for an ingredient under a manifest (`…/ingredients/{instance_id}`).
fn ingredient_stub_tree_id(
    parent_manifest_label: &str,
    ing: &IngredientSummary,
    result: &VerifyResult,
) -> Option<String> {
    let prefix = manifest_tree_section_id(parent_manifest_label, result)?;
    Some(format!("{prefix}/ingredients/{}", ing.instance_id))
}

#[component]
fn IngredientStubDiagram(
    ing: IngredientSummary,
    parent_manifest_label: String,
    result: VerifyResult,
    active_label: String,
    expanded: Signal<HashSet<String>>,
    highlighted: Signal<Option<String>>,
) -> Element {
    let title = ing
        .title
        .clone()
        .unwrap_or_else(|| "(ingredient)".to_string());
    let subtitle = format!("{} · {}", ing.relationship, ing.instance_id);
    let tree_id = ingredient_stub_tree_id(&parent_manifest_label, &ing, &result);
    let click_id = tree_id.clone();
    let active_tid = manifest_tree_section_id(&active_label, &result);

    rsx! {
        div { class: "mrel-subtree mrel-ingredient-stub",
            div {
                class: "mrel-node-click",
                onclick: move |_| {
                    if let Some(ref id) = click_id {
                        let mut exp = expanded.write();
                        exp.insert("root".to_string());
                        for anc in ancestor_ids(id) {
                            exp.insert(anc);
                        }
                        exp.insert(id.clone());
                        highlighted.set(Some(id.clone()));
                    }
                },
                ManifestDiagramNode {
                    title: title,
                    subtitle: subtitle,
                    thumb: None,
                    severity: ManifestNodeSeverity::Ok,
                    is_active: false,
                    is_selected: diagram_manifest_is_selected(&tree_id, &active_tid, &highlighted.read().clone()),
                }
            }
        }
    }
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

/// Tree label for one JSON array element. C2PA `actions` entries are objects with an
/// `action` key (e.g. `c2pa.opened`); show that instead of `[0]`, `[1]`, … when present.
fn array_item_label(element: &Value, index: usize) -> String {
    match element {
        Value::Object(map) => {
            if let Some(v) = map.get("action") {
                match v {
                    Value::String(s) => s.clone(),
                    Value::Null => format!("[{index}]"),
                    other => other.to_string(),
                }
            } else {
                format!("[{index}]")
            }
        }
        _ => format!("[{index}]"),
    }
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
                let item_label = array_item_label(child, i);
                build_json_rows(rows, sections, child, &format!("{id}/{i}"), &item_label, depth + 1);
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
    if let Some(cv) = m.claim_version {
        push_leaf(
            rows,
            format!("{prefix}/claim_version"),
            "claim_version",
            depth,
            cv.to_string(),
        );
    }
    if let Some(cg) = &m.claim_generator {
        push_leaf(rows, format!("{prefix}/claim_generator"), "claim_generator", depth, cg);
    }
    if let Some(cgi) = &m.claim_generator_info {
        let cgi_id = format!("{prefix}/claim_generator_info");
        push_section(rows, sections, &cgi_id, format!("claim_generator_info [{}]", cgi.len()), depth);
        for (i, item) in cgi.iter().enumerate() {
            let item_label = array_item_label(item, i);
            build_json_rows(rows, sections, item, &format!("{cgi_id}/{i}"), &item_label, depth + 1);
        }
    }
    push_leaf(rows, format!("{prefix}/instance_id"), "instance_id", depth, &m.instance_id);

    if m.issuer.is_some()
        || m.common_name.is_some()
        || m.cert_serial_number.is_some()
        || m.signing_time.is_some()
        || m.signature_alg.is_some()
        || m.revocation_status.is_some()
    {
        let sig = format!("{prefix}/signature_info");
        push_section(rows, sections, &sig, "signature_info {…}", depth);
        if let Some(iss) = &m.issuer {
            push_leaf(rows, format!("{sig}/issuer"), "issuer", depth + 1, iss);
        }
        if let Some(cn) = &m.common_name {
            push_leaf(rows, format!("{sig}/common_name"), "common_name", depth + 1, cn);
        }
        if let Some(sn) = &m.cert_serial_number {
            push_leaf(
                rows,
                format!("{sig}/cert_serial_number"),
                "cert_serial_number",
                depth + 1,
                sn,
            );
        }
        if let Some(ts) = &m.signing_time {
            push_leaf(rows, format!("{sig}/time"), "time", depth + 1, ts);
        }
        if let Some(alg) = &m.signature_alg {
            push_leaf(rows, format!("{sig}/alg"), "alg", depth + 1, alg);
        }
        if let Some(rs) = m.revocation_status {
            push_leaf(
                rows,
                format!("{sig}/revocation_status"),
                "revocation_status",
                depth + 1,
                rs.to_string(),
            );
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
    // `push_recent` persists ~/.c2pa-tool/recents.json. We do **not** rebuild the native
    // File ▸ Recent Files menu at runtime: muda/AppKit crashes when removing items from the
    // attached submenu after launch (`MenuChild::remove_inner` / `panic_cannot_unwind`).
    // The OS menu reflects recents from the last app start; the in-app recents signal is always current.
    let mut open_file = move |path: String| {
        info!(target: "c2pa_sample_app::ui::verify", "Opening file for verification: {path}");
        file.set(Some(path.clone()));
        let verify_result = verify_embedded_manifest(&path);
        debug!(target: "c2pa_sample_app::ui::verify", "Verification complete, state: {:?}", verify_result.state);
        push_recent(&path, &mut recents.write());
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
                            info!(target: "c2pa_sample_app::ui::verify", "Browse dialog opened");
                            spawn(async move {
                                if let Some(handle) = rfd::AsyncFileDialog::new().pick_file().await {
                                    let path_str = handle.path().to_string_lossy().to_string();
                                    open_file(path_str);
                                } else {
                                    debug!(target: "c2pa_sample_app::ui::verify", "Browse dialog cancelled");
                                }
                            });
                        },
                        "Browse"
                    }
                }

                if let Some(f) = file.read().clone() {
                    div { class: "file-selected", "✓ {f}" }
                }


                // ── Validation ────────────────────────────────────────────────
                if let Some(res) = result.read().as_ref() {
                    {
                        let (card_class, summary_class, state_label) = match res.state {
                            VerifyValidationState::Trusted => (
                                "card validation-card validation-card-trusted",
                                "validation-summary validation-summary-trusted",
                                "TRUSTED",
                            ),
                            VerifyValidationState::Valid => (
                                "card validation-card validation-card-valid",
                                "validation-summary validation-summary-valid",
                                "VALID",
                            ),
                            VerifyValidationState::Invalid => (
                                "card validation-card validation-card-error",
                                "validation-summary validation-summary-error",
                                "INVALID",
                            ),
                            VerifyValidationState::NoManifest => (
                                "card validation-card validation-card-error",
                                "validation-summary validation-summary-error",
                                "NO MANIFEST",
                            ),
                        };
                        let issuer   = res.manifest.as_ref().and_then(|m| m.issuer.clone());
                        let sig_time = res.manifest.as_ref().and_then(|m| m.signing_time.clone());
                        rsx! {
                            div { class: "{card_class}",
                                div { class: "card-title", "Validation" }
                                div { class: "{summary_class}",
                                    span { class: "validation-summary-dot" }
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
                if let Some(res) = result.read().as_ref().cloned() {
                    if res.manifest.is_some() {
                        ManifestRelationshipDiagram {
                            result: res,
                            expanded: expanded,
                            highlighted: highlighted,
                        }
                    }
                }
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
                                                            highlighted.set(Some(row_id.clone()));
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
                                                                        if let Some(target_id) = find_ingredient_row_id(res, &ing_label, &row_id) {
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
