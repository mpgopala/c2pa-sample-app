mod commands;
mod logger;

use commands::*;

pub fn run() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("trace")))
        .with(logger::UiLogLayer)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            verify_asset,
            sign_asset_cmd,
            add_manifest_cmd,
            load_recents_cmd,
            push_recent_cmd,
            drain_logs_cmd,
            load_signer_prefs_cmd,
            save_signer_prefs_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
