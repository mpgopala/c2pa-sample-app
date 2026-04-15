#![recursion_limit = "256"]
mod app;
mod app_name;
mod logger;
mod menu;
mod pages;

#[cfg(target_os = "macos")]
mod macos_display_name;

use app_name::APP_DISPLAY_NAME;

fn main() {
    #[cfg(target_os = "macos")]
    macos_display_name::set(APP_DISPLAY_NAME);

    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")))
        .with(logger::UiLogLayer)
        .init();
    let recents = model::recents::load_recents();
    let app_menu = menu::build_app_menu(&recents);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::default()
                .with_menu(Some(app_menu))
                .with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title(APP_DISPLAY_NAME)
                        .with_always_on_top(false),
                ),
        )
        .launch(app::App);
}
