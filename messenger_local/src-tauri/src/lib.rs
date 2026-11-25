mod commands;
mod core;
mod modules;

use std::sync::Arc;

use tauri::{Manager, WindowEvent};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::core::{state::IslandStore, suspend::SuspendState, tray, window_manager};
use crate::modules::{
    antigravity::{Antigravity, AntigravitySession},
    discord::Discord,
    gemini::Gemini,
    messenger::Messenger,
    spotify::Spotify,
    IslandModule, ModuleRegistry,
};

fn register_shortcuts<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    let shortcuts = [
        Shortcut::new(Some(Modifiers::CONTROL), Code::Backquote),
        Shortcut::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::Backquote),
    ];
    for shortcut in shortcuts {
        let handle = app.clone();
        if let Err(e) = app.global_shortcut().on_shortcut(shortcut, move |_app, _sc, event| {
            if event.state() == ShortcutState::Pressed {
                let _ = window_manager::toggle_expanded(&handle);
            }
        }) {
            eprintln!("nie udało się zarejestrować skrótu {shortcut:?}: {e}");
        }
    }

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
    unsafe {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            crate::modules::BROWSER_ARGS,
        );
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let messenger = Arc::new(Messenger::new());
            let discord = Arc::new(Discord::new());
            let spotify = Arc::new(Spotify::new());
            let gemini = Arc::new(Gemini::new());
            let mut modules: Vec<Arc<dyn IslandModule>> =
                vec![messenger, discord, spotify, gemini];
            if crate::modules::antigravity::is_available() {
                modules.push(Arc::new(Antigravity::new()));
            }
            app.manage(ModuleRegistry::new(modules));
            app.manage(IslandStore::default());
            app.manage(SuspendState::new());
            app.manage(AntigravitySession::default());

            window_manager::create(app.handle())?;
            tray::setup(app.handle())?;
            register_shortcuts(app.handle());
            core::fullscreen_guard::spawn(app.handle().clone());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_modules,
            commands::get_state,
            commands::set_expanded,
            commands::set_large,
            commands::activate_module,
            commands::cycle_module,
            commands::set_user_hidden,
            commands::close_module,
            commands::start_module,
            commands::ask_antigravity,
            commands::reset_antigravity,
            commands::pick_antigravity_files,
            commands::pick_antigravity_folder,
        ])
        .on_window_event(|window, event| {
            if window.label() != window_manager::ISLAND_LABEL {
                return;
            }
            match event {
                // Wyspa nie ma paska tytułu; zamknięcie kończy się tylko przez zasobnik („Zakończ”).
                WindowEvent::CloseRequested { api, .. } => api.prevent_close(),
                // Zwinięcie po kliknięciu poza wyspą (pomiń gdy otwarte jest okno wyboru plików lub udostępniania ekranu).
                WindowEvent::Focused(false) => {
                    window_manager::handle_focus_lost(window.app_handle());
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
