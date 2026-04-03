use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
enum ManifestMode { Embed, External, Fragmented }

#[component]
pub fn SignPage() -> Element {
    let mut file: Signal<Option<String>> = use_signal(|| None);
    let mut mode = use_signal(|| ManifestMode::Embed);
    let mut title = use_signal(|| String::new());
    let mut format = use_signal(|| String::new());
    let mut assertions: Signal<Vec<String>> = use_signal(Vec::new);
    let mut ingredients: Signal<Vec<String>> = use_signal(Vec::new);
    let mut cert = use_signal(|| String::new());
    let mut key = use_signal(|| String::new());

    rsx! {
        div { class: "page-title", "Sign Asset" }
        div { class: "two-panel",
            // Left panel
            div { class: "panel-left",
                div { class: "drop-zone",
                    p { "Drop file here or" }
                    button { class: "btn btn-sm", onclick: move |_| file.set(Some("example-video.mp4".into())), "Browse" }
                }
                if let Some(f) = file.read().clone() {
                    div { class: "file-selected", "✓ {f}" }
                }
                div { class: "card",
                    div { class: "card-title", "Options" }
                    div { class: "radio-group",
                        label {
                            input { r#type: "radio", checked: *mode.read() == ManifestMode::Embed, onchange: move |_| mode.set(ManifestMode::Embed) }
                            " Embed manifest"
                        }
                        label {
                            input { r#type: "radio", checked: *mode.read() == ManifestMode::External, onchange: move |_| mode.set(ManifestMode::External) }
                            " External manifest"
                        }
                        label {
                            input { r#type: "radio", checked: *mode.read() == ManifestMode::Fragmented, onchange: move |_| mode.set(ManifestMode::Fragmented) }
                            " Fragmented"
                        }
                    }
                }
            }
            // Right panel
            div { class: "panel-right",
                div { class: "card",
                    div { class: "card-title", "Manifest Definition" }
                    div { class: "field",
                        label { "Title" }
                        input { r#type: "text", value: "{title}", placeholder: "My Asset", oninput: move |e| title.set(e.value()) }
                    }
                    div { class: "field",
                        label { "Format" }
                        input { r#type: "text", value: "{format}", placeholder: "video/mp4", oninput: move |e| format.set(e.value()) }
                    }
                    div { class: "card-title", style: "margin-top:12px", "Assertions" }
                    if assertions.read().is_empty() {
                        p { class: "empty-state", "No assertions added" }
                    }
                    for (i, a) in assertions.read().iter().enumerate() {
                        div { class: "list-item",
                            span { "{a}" }
                            button { class: "btn btn-sm btn-danger", onclick: move |_| { assertions.write().remove(i); }, "✕" }
                        }
                    }
                    div { class: "add-row",
                        button { class: "btn btn-sm", onclick: move |_| assertions.write().push("c2pa.actions".into()), "+ Add assertion" }
                    }
                }
                div { class: "card",
                    div { class: "card-title", "Ingredients" }
                    if ingredients.read().is_empty() {
                        p { class: "empty-state", "No ingredients added" }
                    }
                    for (i, ing) in ingredients.read().iter().enumerate() {
                        div { class: "list-item",
                            span { "{ing}" }
                            button { class: "btn btn-sm btn-danger", onclick: move |_| { ingredients.write().remove(i); }, "✕" }
                        }
                    }
                    div { class: "add-row",
                        button { class: "btn btn-sm", onclick: move |_| ingredients.write().push("ingredient.jpg".into()), "+ Add" }
                    }
                }
                div { class: "card",
                    div { class: "card-title", "Signer" }
                    div { class: "field",
                        label { "Certificate" }
                        div { class: "inline-row",
                            input { r#type: "text", value: "{cert}", placeholder: "cert.pem", style: "flex:1", oninput: move |e| cert.set(e.value()) }
                            button { class: "btn btn-sm", onclick: move |_| cert.set("cert.pem".into()), "Browse" }
                        }
                    }
                    div { class: "field",
                        label { "Private Key" }
                        div { class: "inline-row",
                            input { r#type: "text", value: "{key}", placeholder: "key.pem", style: "flex:1", oninput: move |e| key.set(e.value()) }
                            button { class: "btn btn-sm", onclick: move |_| key.set("key.pem".into()), "Browse" }
                        }
                    }
                }
                button { class: "btn btn-primary btn-full", "Sign Asset" }
            }
        }
    }
}
