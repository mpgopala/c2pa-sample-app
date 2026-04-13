use c2pa_sample_app::model::recents::{load_recents, RecentEntry};
use dioxus::desktop::use_muda_event_handler;
use dioxus::document::eval;
use dioxus::prelude::*;
use crate::logger::{drain_logs, set_log_level, LogEntry, LogLevel};
use crate::menu::{log_level_for_id, path_for_id, set_active_log_level, set_log_pane_checked, MENU_TOGGLE_LOG};
use tracing::info;
use crate::pages::{settings::SettingsPage, sign::SignPage, verify::VerifyPage};

#[derive(Clone, PartialEq)]
pub enum Page {
    Sign,
    Verify,
    Settings,
}

#[component]
pub fn App() -> Element {
    let mut page = use_signal(|| Page::Sign);
    let _recents: Signal<Vec<RecentEntry>> =
        use_context_provider(|| Signal::new(load_recents()));
    let mut pending_open: Signal<Option<String>> =
        use_context_provider(|| Signal::new(None::<String>));

    // ── Logging state ─────────────────────────────────────────────────────────
    let mut log_visible: Signal<bool> = use_signal(|| false);
    let mut log_entries: Signal<Vec<LogEntry>> =
        use_context_provider(|| Signal::new(Vec::<LogEntry>::new()));

    // ── Log pane resize state ─────────────────────────────────────────────────
    let mut log_height: Signal<u32>       = use_signal(|| 220);
    let mut dragging: Signal<bool>        = use_signal(|| false);
    let mut drag_start_y: Signal<f64>     = use_signal(|| 0.0);
    let mut drag_start_h: Signal<u32>     = use_signal(|| 220);

    // Poll the static log buffer every 200 ms and append new entries to signal.
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
            let new = drain_logs();
            if !new.is_empty() {
                let mut entries = log_entries.write();
                entries.extend(new);
                if entries.len() > 500 {
                    let drain_count = entries.len() - 500;
                    entries.drain(0..drain_count);
                }
            }
        }
    });

    // ── Native menu events ────────────────────────────────────────────────────
    use_muda_event_handler(move |event| {
        let id = event.id().0.as_str();
        if id == "file-open" {
            page.set(Page::Verify);
            spawn(async move {
                if let Some(handle) = rfd::AsyncFileDialog::new().pick_file().await {
                    pending_open.set(Some(handle.path().to_string_lossy().to_string()));
                }
            });
        } else if id == MENU_TOGGLE_LOG {
            let next = !*log_visible.read();
            log_visible.set(next);
            set_log_pane_checked(next);
        } else if let Some(level) = log_level_for_id(id) {
            set_log_level(&level);
            set_active_log_level(&level);
        } else if let Some(path) = path_for_id(id) {
            page.set(Page::Verify);
            pending_open.set(Some(path));
        }
    });

    rsx! {
        style { {include_str!("styles.css")} }
        div { class: "root",

            // ── Drag overlay ──────────────────────────────────────────────────
            // Covers the full viewport while the user is dragging the resize
            // handle so that fast mouse movement never escapes the hit-target.
            if *dragging.read() {
                div {
                    style: "position:fixed;inset:0;z-index:9999;cursor:ns-resize;",
                    onmousemove: move |e| {
                        let dy = e.client_coordinates().y - *drag_start_y.read();
                        let new_h = (*drag_start_h.read() as f64 - dy)
                            .max(80.0).min(600.0) as u32;
                        log_height.set(new_h);
                    },
                    onmouseup: move |_| dragging.set(false),
                }
            }

            nav { class: "nav",
                span { class: "nav-brand", "C2PA Tool" }
                button {
                    class: if *page.read() == Page::Sign { "nav-tab active" } else { "nav-tab" },
                    onclick: move |_| {
                        info!(target: "c2pa_tool::nav", "Navigated to Sign");
                        page.set(Page::Sign);
                    },
                    "Sign"
                }
                button {
                    class: if *page.read() == Page::Verify { "nav-tab active" } else { "nav-tab" },
                    onclick: move |_| {
                        info!(target: "c2pa_tool::nav", "Navigated to Verify");
                        page.set(Page::Verify);
                    },
                    "Verify"
                }
                button {
                    class: if *page.read() == Page::Settings { "nav-tab active" } else { "nav-tab" },
                    onclick: move |_| {
                        info!(target: "c2pa_tool::nav", "Navigated to Settings");
                        page.set(Page::Settings);
                    },
                    "Settings"
                }
            }

            // ── Main content (top) + log pane (bottom) ────────────────────────
            // LogPane is always in the DOM; "log-hidden" hides it via CSS.
            div {
                class: if *log_visible.read() { "content-and-log" } else { "content-and-log log-hidden" },

                div { class: "page-content",
                    match *page.read() {
                        Page::Sign     => rsx! { SignPage {} },
                        Page::Verify   => rsx! { VerifyPage {} },
                        Page::Settings => rsx! { SettingsPage {} },
                    }
                }

                // Drag handle — sits between page content and log pane.
                div {
                    class: "log-resize-handle",
                    onmousedown: move |e| {
                        e.prevent_default();
                        dragging.set(true);
                        drag_start_y.set(e.client_coordinates().y);
                        drag_start_h.set(*log_height.read());
                    },
                }

                LogPane { entries: log_entries, height: *log_height.read() }
            }
        }
    }
}

// ── Log pane component ────────────────────────────────────────────────────────

#[component]
fn LogPane(entries: Signal<Vec<LogEntry>>, height: u32) -> Element {
    let mut auto_scroll = use_signal(|| true);
    let mut filter_text = use_signal(String::new);
    let mut filter_level: Signal<Option<LogLevel>> = use_signal(|| None);

    // Scroll the entries container to the bottom whenever new entries arrive,
    // as long as auto-scroll is enabled.  The `entries.read().len()` read
    // creates a reactive dependency so this fires on every new batch.
    use_effect(move || {
        let _len = entries.read().len();
        if *auto_scroll.read() {
            let _ = eval(
                "var e=document.getElementById('log-entries-container');\
                 if(e) e.scrollTop=e.scrollHeight;"
            );
        }
    });

    let format_ts = |ts_ms: u64| -> String {
        let secs = ts_ms / 1000;
        let ms   = ts_ms % 1000;
        let h    = (secs / 3600) % 24;
        let m    = (secs / 60) % 60;
        let s    = secs % 60;
        format!("{h:02}:{m:02}:{s:02}.{ms:03}")
    };

    // Apply text + level filters (oldest-first order is preserved from the Vec).
    let visible: Vec<LogEntry> = {
        let text = filter_text.read().to_lowercase();
        let cap  = filter_level.read().clone();
        entries.read().iter()
            .filter(|e| {
                if let Some(c) = &cap { if e.level > *c { return false; } }
                if !text.is_empty() {
                    let m = e.message.to_lowercase();
                    let t = e.target.to_lowercase();
                    if !m.contains(text.as_str()) && !t.contains(text.as_str()) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect()
    };

    rsx! {
        div { class: "log-pane", style: "height: {height}px",

            // ── Header row ────────────────────────────────────────────────────
            div { class: "log-header",
                span { class: "log-header-title", "Log" }
                div { class: "log-header-actions",
                    label { class: "log-autoscroll-label",
                        input {
                            r#type: "checkbox",
                            checked: *auto_scroll.read(),
                            onchange: move |e| auto_scroll.set(e.checked())
                        }
                        " Auto-scroll"
                    }
                    button {
                        class: "btn btn-sm",
                        onclick: move |_| entries.write().clear(),
                        "Clear"
                    }
                }
            }

            // ── Filter row ────────────────────────────────────────────────────
            div { class: "log-filter-row",
                input {
                    class: "log-filter-input",
                    r#type: "text",
                    placeholder: "Filter logs…",
                    value: "{filter_text}",
                    oninput: move |e| filter_text.set(e.value()),
                }
                select {
                    class: "log-filter-select",
                    onchange: move |e| {
                        filter_level.set(match e.value().as_str() {
                            "error" => Some(LogLevel::Error),
                            "warn"  => Some(LogLevel::Warn),
                            "info"  => Some(LogLevel::Info),
                            "debug" => Some(LogLevel::Debug),
                            "trace" => Some(LogLevel::Trace),
                            _       => None,
                        });
                    },
                    option { value: "all",  "All levels" }
                    option { value: "trace", "Trace+" }
                    option { value: "debug", "Debug+" }
                    option { value: "info",  "Info+" }
                    option { value: "warn",  "Warn+" }
                    option { value: "error", "Error only" }
                }
            }

            // ── Entries (oldest → newest, auto-scrolls to bottom) ─────────────
            div {
                class: "log-entries",
                id: "log-entries-container",
                if visible.is_empty() {
                    div { class: "log-empty",
                        if entries.read().is_empty() { "No log entries yet." }
                        else { "No entries match the current filter." }
                    }
                }
                for entry in visible.iter() {
                    div { class: "log-row {entry.css_class()}",
                        span { class: "log-ts",    "{format_ts(entry.ts_ms)}" }
                        span { class: "log-level {entry.css_class()}-badge", "{entry.level_label()}" }
                        span { class: "log-target", "{entry.target}" }
                        span { class: "log-msg",   "{entry.message}" }
                    }
                }
            }
        }
    }
}
