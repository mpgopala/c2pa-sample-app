use c2pa_sample_app::model::recents::{load_recents, RecentEntry};
use dioxus::prelude::*;
use crate::pages::{sign::SignPage, verify::VerifyPage, settings::SettingsPage};

#[derive(Clone, PartialEq)]
pub enum Page { Sign, Verify, Settings }

#[component]
pub fn App() -> Element {
    let mut page = use_signal(|| Page::Sign);

    // Shared recents state — loaded from disk once, mutated by any page.
    use_context_provider(|| Signal::new(load_recents() as Vec<RecentEntry>));

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
