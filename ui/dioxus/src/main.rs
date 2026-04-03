#![recursion_limit = "256"]
mod app;
mod logger;
mod menu;
mod pages;

fn main() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")))
        .with(logger::UiLogLayer)
        .init();
    let recents = c2pa_sample_app::model::recents::load_recents();
    let app_menu = menu::build_app_menu(&recents);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::default()
                .with_menu(Some(app_menu))
                .with_window(
                    dioxus::desktop::WindowBuilder::new().with_always_on_top(false),
                ),
        )
        .launch(app::App);
}
