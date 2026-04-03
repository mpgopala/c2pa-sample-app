use c2pa_sample_app::model::recents::RecentEntry;
use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

// Submenu is not Send+Sync, so it lives in a thread-local (UI always runs on the main thread).
thread_local! {
    static RECENTS_SUBMENU: RefCell<Option<Submenu>> = const { RefCell::new(None) };
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
