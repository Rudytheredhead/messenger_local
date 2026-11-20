use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager, Runtime, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_notification::NotificationExt;

const ALLOWED_DOMAINS: &[&str] = &[
    "facebook.com",
    "messenger.com",
    "fbcdn.net",
    "fbsbx.com",
];

fn is_allowed_url(url: &str) -> bool {
    if let Ok(parsed) = url::Url::parse(url) {
        if parsed.scheme() != "https" {
            return false;
        }
        if let Some(host) = parsed.host_str() {
            return ALLOWED_DOMAINS.iter().any(|domain| {
                host == *domain || host.ends_with(&format!(".{}", domain))
            });
        }
    }
    false
}

fn setup_tray<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "Otwórz", true, None::<&str>)?;
    let logout_item = MenuItem::with_id(app, "logout", "Wyloguj / Wyczyść sesję", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &logout_item, &quit_item])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().unwrap())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "logout" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.eval("window.location.href = 'https://www.facebook.com/logout.php';");
                }
                if let Ok(data_dir) = app.path().app_data_dir() {
                    let webview_dir = data_dir.join("webview");
                    if webview_dir.exists() {
                        let _ = std::fs::remove_dir_all(&webview_dir);
                    }
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            setup_tray(app)?;
            Ok(())
        })
        .on_navigation(|window, url| {
            let url_str = url.as_str();
            if is_allowed_url(url_str) {
                return true;
            }
            let _ = window.app_handle().opener().open_url(url_str, None::<&str>);
            false
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}