use std::time::Duration;

use tauri::{AppHandle, Manager, State};

use crate::core::window_manager::{self, IslandState};
use crate::modules::{
    antigravity::{self, AntigravityReply, AntigravitySession},
    ModuleInfo, ModuleRegistry,
};

#[tauri::command]
pub fn list_modules(registry: State<'_, ModuleRegistry>) -> Vec<ModuleInfo> {
    registry.list()
}

#[tauri::command]
pub fn get_state(app: AppHandle) -> IslandState {
    window_manager::snapshot(&app)
}

#[tauri::command]
pub fn set_expanded(app: AppHandle, expanded: bool) -> Result<(), String> {
    window_manager::set_expanded(&app, expanded).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_large(app: AppHandle, large: bool) -> Result<(), String> {
    window_manager::set_large(&app, large).map_err(|e| e.to_string())
}

/// Aktywacja modułu: sprawdza, czy istnieje, przełącza na niego widok i rozwija wyspę.
#[tauri::command]
pub fn activate_module(app: AppHandle, module_id: String) -> Result<(), String> {
    let registry = app.state::<ModuleRegistry>();
    if registry.get(&module_id).is_none() {
        return Err(format!("nieznany moduł: {module_id}"));
    }
    window_manager::activate_module(&app, &module_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn cycle_module(app: AppHandle, delta: i32) -> Result<(), String> {
    window_manager::cycle_module(&app, delta).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_user_hidden(app: AppHandle, hidden: bool) -> Result<(), String> {
    window_manager::set_user_hidden(&app, hidden).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_module(app: AppHandle, module_id: String) -> Result<(), String> {
    window_manager::close_module(&app, &module_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_module(app: AppHandle, module_id: String) -> Result<(), String> {
    window_manager::start_module(&app, &module_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn ask_antigravity(
    app: AppHandle,
    prompt: String,
    model: Option<String>,
    effort: Option<String>,
    conversation_id: Option<String>,
    paths: Vec<String>,
) -> Result<AntigravityReply, String> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<AntigravitySession>().ask(
            &prompt,
            model.as_deref(),
            effort.as_deref(),
            conversation_id.as_deref(),
            &paths,
        )
    })
    .await
    .map_err(|e| format!("Błąd zadania Antigravity: {e}"))?
}

#[tauri::command]
pub fn reset_antigravity(app: AppHandle) -> Result<(), String> {
    app.state::<AntigravitySession>().reset();
    Ok(())
}

#[tauri::command]
pub async fn pick_antigravity_files(app: AppHandle) -> Result<Vec<String>, String> {
    window_manager::set_dialog_open(&app, true);
    let result = tauri::async_runtime::spawn_blocking(antigravity::pick_files_dialog)
        .await
        .map_err(|e| format!("Błąd wyboru plików: {e}"))?;
    let app_restore = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        window_manager::set_dialog_open(&app_restore, false);
    });
    result
}

#[tauri::command]
pub async fn pick_antigravity_folder(app: AppHandle) -> Result<Option<String>, String> {
    window_manager::set_dialog_open(&app, true);
    let result = tauri::async_runtime::spawn_blocking(antigravity::pick_folder_dialog)
        .await
        .map_err(|e| format!("Błąd wyboru folderu: {e}"))?;
    let app_restore = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        window_manager::set_dialog_open(&app_restore, false);
    });
    result
}
