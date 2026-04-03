use c2pa_sample_app::model::recents::{load_recents, RecentEntry};
use dioxus::desktop::use_muda_event_handler;
use dioxus::prelude::*;
use crate::menu::path_for_id;
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

    // Handle native menu bar events.
    use_muda_event_handler(move |event| {
        let id = event.id().0.as_str();
        if id == "file-open" {
            page.set(Page::Verify);
            spawn(async move {
                if let Some(handle) = rfd::AsyncFileDialog::new().pick_file().await {
                    pending_open.set(Some(handle.path().to_string_lossy().to_string()));
                }
            });
        } else if let Some(path) = path_for_id(id) {
            page.set(Page::Verify);
            pending_open.set(Some(path));
        }
    });

    rsx! {
        style { {include_str!("styles.css")} }
        div { class: "root",
            nav { class: "nav",
                span { class: "nav-brand", "C2PA Tool" }
                button {
                    class: if *page.read() == Page::Sign { "nav-tab active" } else { "nav-tab" },
                    onclick: move |_| page.set(Page::Sign),
                    "Sign"
                }
                button {
                    class: if *page.read() == Page::Verify { "nav-tab active" } else { "nav-tab" },
                    onclick: move |_| page.set(Page::Verify),
                    "Verify"
                }
                button {
                    class: if *page.read() == Page::Settings { "nav-tab active" } else { "nav-tab" },
                    onclick: move |_| page.set(Page::Settings),
                    "Settings"
                }
            }
            div { class: "page-content",
                match *page.read() {
                    Page::Sign => rsx! { SignPage {} },
                    Page::Verify => rsx! { VerifyPage {} },
                    Page::Settings => rsx! { SettingsPage {} },
                }
            }
        }
    }
}
