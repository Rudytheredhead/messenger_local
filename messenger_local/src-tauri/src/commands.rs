use tauri::{AppHandle, Manager, State};

use crate::core::window_manager::{self, IslandState};
use crate::modules::{ModuleInfo, ModuleRegistry};

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

/// Aktywacja modułu: sprawdza, czy istnieje, i rozwija wyspę. Obecnie jedyny moduł (Messenger)
/// ma natywny WebView; przyszłe moduły HTML/JS będą przełączane w kontenerze wyspy.
#[tauri::command]
pub fn activate_module(app: AppHandle, module_id: String) -> Result<(), String> {
    let registry = app.state::<ModuleRegistry>();
    if registry.get(&module_id).is_none() {
        return Err(format!("nieznany moduł: {module_id}"));
    }
    window_manager::set_expanded(&app, true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_user_hidden(app: AppHandle, hidden: bool) -> Result<(), String> {
    window_manager::set_user_hidden(&app, hidden).map_err(|e| e.to_string())
}
