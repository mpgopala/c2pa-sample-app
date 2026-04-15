use model::recents::RecentEntry;
use crate::logger::LogLevel;
#[cfg(target_os = "macos")]
use crate::app_name::APP_DISPLAY_NAME;
#[cfg(target_os = "macos")]
use muda::AboutMetadata;
use muda::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// Submenus / check items are not Send+Sync — keep them in thread-locals.
thread_local! {
    static RECENTS_SUBMENU: RefCell<Option<Submenu>> = const { RefCell::new(None) };
    static LOG_PANE_ITEM: RefCell<Option<CheckMenuItem>> = const { RefCell::new(None) };
    static LOG_LEVEL_ITEMS: RefCell<Vec<(LogLevel, CheckMenuItem)>> = const { RefCell::new(Vec::new()) };
}

pub const MENU_TOGGLE_LOG: &str = "view-toggle-log";

/// Map a menu event ID to a LogLevel; returns None for non-level IDs.
pub fn log_level_for_id(id: &str) -> Option<LogLevel> {
    match id {
        "log-level-trace" => Some(LogLevel::Trace),
        "log-level-debug" => Some(LogLevel::Debug),
        "log-level-info"  => Some(LogLevel::Info),
        "log-level-warn"  => Some(LogLevel::Warn),
        "log-level-error" => Some(LogLevel::Error),
        _                 => None,
    }
}

/// Sync the native check state of all Log Level menu items.
pub fn set_active_log_level(active: &LogLevel) {
    LOG_LEVEL_ITEMS.with(|cell| {
        for (level, item) in cell.borrow().iter() {
            item.set_checked(level == active);
        }
    });
}

/// Sync the native check state of the "Show Log Pane" menu item.
pub fn set_log_pane_checked(checked: bool) {
    LOG_PANE_ITEM.with(|cell| {
        if let Some(item) = cell.borrow().as_ref() {
            item.set_checked(checked);
        }
    });
}

static RECENT_ID_MAP: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn id_map() -> &'static Mutex<HashMap<String, String>> {
    RECENT_ID_MAP.get_or_init(|| Mutex::new(HashMap::new()))
}

fn id_for(path: &str) -> String {
    format!("recent::{path}")
}

/// Decode a menu event ID back to a file path; returns `None` for non-recent IDs.
pub fn path_for_id(id: &str) -> Option<String> {
    id.strip_prefix("recent::").map(str::to_string)
}

fn remove_all_items(sub: &Submenu) {
    while sub.remove_at(0).is_some() {}
}

fn populate(sub: &Submenu, recents: &[RecentEntry]) {
    let mut map = id_map().lock().unwrap();
    map.clear();
    for entry in recents {
        let id = id_for(&entry.path);
        map.insert(id.clone(), entry.path.clone());
        let _ = sub.append(&MenuItem::with_id(id, &entry.name, true, None));
    }
}

/// Build the full native menu bar. Must be called on the main thread before launch.
pub fn build_app_menu(recents: &[RecentEntry]) -> Menu {
    let menu = Menu::new();

    // ── Application (macOS) ───────────────────────────────────────────────────
    // First submenu title is the name shown in the menu bar (not the binary name `ui`).
    #[cfg(target_os = "macos")]
    {
        // Explicit labels: muda’s defaults use NSRunningApplication.localizedName (“ui” for cargo run).
        let about_lbl = format!("About {}", APP_DISPLAY_NAME);
        let hide_lbl = format!("Hide {}", APP_DISPLAY_NAME);
        let quit_lbl = format!("Quit {}", APP_DISPLAY_NAME);

        let app_menu = Submenu::new(APP_DISPLAY_NAME, true);
        let about_meta = AboutMetadata {
            name: Some(APP_DISPLAY_NAME.to_string()),
            ..Default::default()
        };
        let _ = app_menu.append(&PredefinedMenuItem::about(Some(&about_lbl), Some(about_meta)));
        let _ = app_menu.append(&PredefinedMenuItem::separator());
        let _ = app_menu.append(&PredefinedMenuItem::services(None));
        let _ = app_menu.append(&PredefinedMenuItem::separator());
        let _ = app_menu.append(&PredefinedMenuItem::hide(Some(&hide_lbl)));
        let _ = app_menu.append(&PredefinedMenuItem::hide_others(None));
        let _ = app_menu.append(&PredefinedMenuItem::show_all(None));
        let _ = app_menu.append(&PredefinedMenuItem::separator());
        let _ = app_menu.append(&PredefinedMenuItem::quit(Some(&quit_lbl)));
        let _ = menu.prepend(&app_menu);
    }

    // ── File ─────────────────────────────────────────────────────────────────
    let file_menu = Submenu::new("File", true);
    let _ = file_menu.append(&MenuItem::with_id("file-open", "Open…", true, None));
    let _ = file_menu.append(&PredefinedMenuItem::separator());

    let recents_sub = Submenu::new("Recent Files", !recents.is_empty());
    if recents.is_empty() {
        let _ = recents_sub.append(&MenuItem::with_id(
            "recents-empty",
            "No Recent Files",
            false,
            None,
        ));
    } else {
        populate(&recents_sub, recents);
    }
    let _ = file_menu.append(&recents_sub);

    let _ = file_menu.append(&PredefinedMenuItem::separator());
    #[cfg(not(target_os = "macos"))]
    let _ = file_menu.append(&PredefinedMenuItem::quit(None));
    let _ = menu.append(&file_menu);

    // Store for later rebuilds.
    RECENTS_SUBMENU.with(|cell| *cell.borrow_mut() = Some(recents_sub));

    // ── Edit ─────────────────────────────────────────────────────────────────
    let edit_menu = Submenu::new("Edit", true);
    let _ = edit_menu.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::select_all(None),
    ]);
    let _ = menu.append(&edit_menu);

    // ── View ─────────────────────────────────────────────────────────────────
    let view_menu = Submenu::new("View", true);
    let log_item = CheckMenuItem::with_id(MENU_TOGGLE_LOG, "Show Log Pane", true, false, None);
    let _ = view_menu.append(&log_item);
    LOG_PANE_ITEM.with(|cell| *cell.borrow_mut() = Some(log_item));

    let _ = view_menu.append(&PredefinedMenuItem::separator());

    let level_sub = Submenu::new("Log Level", true);
    let level_defs: &[(&str, &str, LogLevel)] = &[
        ("log-level-trace", "Trace", LogLevel::Trace),
        ("log-level-debug", "Debug", LogLevel::Debug),
        ("log-level-info",  "Info",  LogLevel::Info),
        ("log-level-warn",  "Warn",  LogLevel::Warn),
        ("log-level-error", "Error", LogLevel::Error),
    ];
    let mut level_items: Vec<(LogLevel, CheckMenuItem)> = Vec::new();
    for (id, label, level) in level_defs {
        let checked = matches!(level, LogLevel::Trace);
        let item = CheckMenuItem::with_id(*id, *label, true, checked, None);
        let _ = level_sub.append(&item);
        level_items.push((level.clone(), item));
    }
    LOG_LEVEL_ITEMS.with(|cell| *cell.borrow_mut() = level_items);
    let _ = view_menu.append(&level_sub);

    let _ = menu.append(&view_menu);

    // ── Window (macOS) ────────────────────────────────────────────────────────
    #[cfg(target_os = "macos")]
    {
        let window_menu = Submenu::new("Window", true);
        let _ = window_menu.append_items(&[
            &PredefinedMenuItem::minimize(None),
            &PredefinedMenuItem::maximize(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::close_window(None),
        ]);
        window_menu.set_as_windows_menu_for_nsapp();
        let _ = menu.append(&window_menu);
    }

    menu
}

/// Rebuild the "Recent Files" submenu to reflect a changed recents list.
/// Must be called from the main thread (e.g., from a Dioxus `use_effect`).
pub fn rebuild_recents_menu(recents: &[RecentEntry]) {
    RECENTS_SUBMENU.with(|cell| {
        let borrow = cell.borrow();
        let Some(sub) = borrow.as_ref() else { return };
        remove_all_items(sub);
        let _ = sub.set_enabled(!recents.is_empty());
        if recents.is_empty() {
            let _ = sub.append(&MenuItem::with_id(
                "recents-empty",
                "No Recent Files",
                false,
                None,
            ));
        } else {
            populate(sub, recents);
        }
    });
}
