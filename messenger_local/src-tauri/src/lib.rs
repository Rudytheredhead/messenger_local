mod commands;
mod core;
mod modules;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::core::{state::IslandStore, suspend::SuspendState, tray, window_manager};
use crate::modules::{messenger::Messenger, IslandModule, ModuleRegistry};

fn register_shortcuts<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let handle = app.clone();
    let result = app.global_shortcut().on_shortcut("ctrl+alt+i", move |_app, _shortcut, event| {
        if event.state() == ShortcutState::Pressed {
            let _ = window_manager::toggle_expanded(&handle);
        }
    });
    if let Err(e) = result {
        // Skrót jest wygodą, nie wymogiem — start aplikacji nie może przez to paść.
        eprintln!("nie udało się zarejestrować skrótu Ctrl+Alt+I: {e}");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let messenger = Arc::new(Messenger::new());
            let modules: Vec<Arc<dyn IslandModule>> = vec![messenger.clone()];
            app.manage(ModuleRegistry::new(modules));
            app.manage(IslandStore::default());
            app.manage(SuspendState::new());

            window_manager::create(app.handle(), messenger)?;
            tray::setup(app.handle())?;
            register_shortcuts(app.handle());
            core::fullscreen_guard::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_modules,
            commands::get_state,
            commands::set_expanded,
            commands::activate_module,
            commands::set_user_hidden,
        ])
        .on_window_event(|window, event| {
            if window.label() != window_manager::ISLAND_LABEL {
                return;
            }
            match event {
                // Wyspa nie ma paska tytułu; zamknięcie kończy się tylko przez zasobnik („Zakończ”).
                WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
                // Zwinięcie po kliknięciu poza wyspą (spec: auto-zwijanie przy utracie fokusu).
                WindowEvent::Focused(false) => {
                    let _ = window_manager::set_expanded(window.app_handle(), false);
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
