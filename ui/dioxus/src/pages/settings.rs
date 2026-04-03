use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
enum ConfigMode { File, Json }

#[component]
pub fn SettingsPage() -> Element {
    let mut trust_lists: Signal<Vec<String>> = use_signal(|| vec!["c2pa-trust-list.pem".into(), "custom-ca.pem".into()]);
    let mut config_mode = use_signal(|| ConfigMode::File);
    let mut config_file = use_signal(|| "config.toml".to_string());
    let mut config_json = use_signal(|| String::new());
    let mut fetch_remote = use_signal(|| true);
    let mut timeout = use_signal(|| 30u32);

    rsx! {
        div { class: "settings-page",
            div { style: "font-size:18px;font-weight:600", "Settings" }

            div { class: "card",
                div { class: "card-title", "Trust Lists" }
                for (i, t) in trust_lists.read().iter().enumerate() {
                    div { class: "trust-item",
                        span { "{t}" }
                        button { class: "btn btn-sm btn-danger", onclick: move |_| { trust_lists.write().remove(i); }, "Remove" }
                    }
                }
                div { class: "add-row",
                    button { class: "btn btn-sm", onclick: move |_| trust_lists.write().push("new-trust.pem".into()), "+ Add" }
                }
            }

            div { class: "card",
                div { class: "card-title", "Configuration" }
                div { class: "radio-group",
                    label {
                        input { r#type: "radio", checked: *config_mode.read() == ConfigMode::File, onchange: move |_| config_mode.set(ConfigMode::File) }
                        " Load from file"
                    }
                    if *config_mode.read() == ConfigMode::File {
                        div { class: "inline-row", style: "margin-left:20px",
                            input { r#type: "text", value: "{config_file}", style: "flex:1", oninput: move |e| config_file.set(e.value()) }
                            button { class: "btn btn-sm", "Browse" }
                        }
                    }
                    label {
                        input { r#type: "radio", checked: *config_mode.read() == ConfigMode::Json, onchange: move |_| config_mode.set(ConfigMode::Json) }
                        " Load from JSON"
                    }
                    if *config_mode.read() == ConfigMode::Json {
                        textarea {
                            value: "{config_json}",
                            placeholder: "{{ \"trust\": {{ ... }} }}",
                            style: "margin-left:20px;width:calc(100% - 20px);height:80px;padding:8px;border:1px solid var(--border);border-radius:6px;font-family:monospace;font-size:12px",
                            oninput: move |e| config_json.set(e.value())
                        }
                    }
                }
            }

            div { class: "card",
                div { class: "card-title", "HTTP Resolution" }
                div { class: "checkbox-group",
                    label {
                        input { r#type: "checkbox", checked: *fetch_remote.read(), onchange: move |e| fetch_remote.set(e.checked()) }
                        " Fetch remote manifests automatically"
                    }
                }
                div { class: "inline-row", style: "margin-top:10px",
                    span { style: "font-size:13px;color:var(--text-secondary)", "Timeout" }
                    input { r#type: "number", value: "{timeout}", min: 1, max: 300, style: "width:64px;padding:6px 8px;border:1px solid var(--border);border-radius:6px", oninput: move |e| { if let Ok(v) = e.value().parse() { timeout.set(v); } } }
                    span { style: "font-size:13px;color:var(--text-secondary)", "seconds" }
                }
            }

            div { class: "settings-actions",
                button { class: "btn", "Reset to Default" }
                button { class: "btn btn-primary", "Save" }
            }
        }
    }
}
