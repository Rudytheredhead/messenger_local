use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    webview::NewWindowResponse,
    AppHandle, Manager, Runtime, WebviewUrl, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;
use url::Url;

const MAIN_LABEL: &str = "main";
const START_URL: &str = "https://www.facebook.com/messages";
/// Podkatalog danych WebView (ciasteczka, cache, local storage) — tylko dla tej aplikacji.
const PROFILE_DIR: &str = "webview-profile";

const ALLOWED_DOMAINS: &[&str] = &[
    "facebook.com",
    "messenger.com",
    "fbcdn.net",
    "fbsbx.com",
];

/// Dozwolone są wyłącznie https, bez danych uwierzytelniających w URL,
/// na standardowym porcie, i tylko dla zweryfikowanych domen Mety (lub ich subdomen).
fn is_allowed_url(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    if !url.username().is_empty() || url.password().is_some() {
        return false;
    }
    if url.port().is_some() {
        return false;
    }
    match url.host_str() {
        Some(host) => ALLOWED_DOMAINS
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}"))),
        None => false,
    }
}

/// Otwiera link w domyślnej przeglądarce systemowej (tylko https).
fn open_external<R: Runtime>(app: &AppHandle<R>, url: &Url) {
    if url.scheme() != "https" {
        return;
    }
    if let Err(e) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("nie udało się otworzyć linku zewnętrznego: {e}");
    }
}

/// Decyzja dla każdej nawigacji najwyższego poziomu w oknie głównym.
/// `true` = pozwól, `false` = zablokuj (obce linki trafiają do przeglądarki).
fn handle_navigation<R: Runtime>(app: &AppHandle<R>, url: &Url) -> bool {
    if url.as_str() == "about:blank" || is_allowed_url(url) {
        return true;
    }
    open_external(app, url);
    false
}

/// Nowe okna (target=_blank, window.open): domeny Mety otwieramy w głównym oknie,
/// resztę w przeglądarce systemowej. Nigdy nie tworzymy dodatkowych okien WebView.
fn handle_new_window<R: Runtime>(app: &AppHandle<R>, url: Url) -> NewWindowResponse<R> {
    if is_allowed_url(&url) {
        let app = app.clone();
        // Nawigacja asynchronicznie, żeby nie wywoływać WebView z jego własnego callbacku.
        tauri::async_runtime::spawn(async move {
            if let Some(window) = app.get_webview_window(MAIN_LABEL) {
                if let Err(e) = window.navigate(url) {
                    eprintln!("nawigacja nowego okna nie powiodła się: {e}");
                }
            }
        });
    } else {
        open_external(app, &url);
    }
    NewWindowResponse::Deny
}

fn create_main_window<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let start_url = Url::parse(START_URL).expect("stały, poprawny URL");
    let profile_dir = app.path().app_local_data_dir()?.join(PROFILE_DIR);

    let nav_handle = app.clone();
    let new_window_handle = app.clone();

    WebviewWindowBuilder::new(app, MAIN_LABEL, WebviewUrl::External(start_url))
        .title("Messenger")
        .inner_size(1200.0, 800.0)
        .min_inner_size(800.0, 600.0)
        .resizable(true)
        .content_protected(true)
        .data_directory(profile_dir)
        .on_navigation(move |url| handle_navigation(&nav_handle, url))
        .on_new_window(move |url, _features| handle_new_window(&new_window_handle, url))
        .build()?;

    Ok(())
}

fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Usuwa ciasteczka, cache i pamięć lokalną WebView (sesja Messengera), potem wraca na stronę logowania.
fn logout<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_LABEL) {
        if let Err(e) = window.clear_all_browsing_data() {
            eprintln!("nie udało się wyczyścić danych przeglądarki: {e}");
        }
        if let Ok(url) = Url::parse(START_URL) {
            let _ = window.navigate(url);
        }
    }
}

fn setup_tray<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "Otwórz", true, None::<&str>)?;
    let logout_item = MenuItem::with_id(app, "logout", "Wyloguj / Wyczyść sesję", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &logout_item, &quit_item])?;

    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("Messenger Local")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "logout" => logout(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            create_main_window(app.handle())?;
            setup_tray(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Zamknięcie okna chowa je do zasobnika zamiast kończyć sesję.
            if window.label() == MAIN_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed(s: &str) -> bool {
        Url::parse(s).map(|u| is_allowed_url(&u)).unwrap_or(false)
    }

    #[test]
    fn allows_meta_https_domains() {
        assert!(allowed("https://www.facebook.com/messages"));
        assert!(allowed("https://facebook.com/"));
        assert!(allowed("https://www.messenger.com/t/1"));
        assert!(allowed("https://scontent.xx.fbcdn.net/v/a.jpg"));
        assert!(allowed("https://video.fbsbx.com/x"));
    }

    #[test]
    fn blocks_http_and_other_schemes() {
        assert!(!allowed("http://www.facebook.com/messages"));
        assert!(!allowed("javascript:alert(1)"));
        assert!(!allowed("file:///C:/Windows/win.ini"));
        assert!(!allowed("data:text/html,hi"));
    }

    #[test]
    fn blocks_lookalike_and_foreign_hosts() {
        assert!(!allowed("https://evilfacebook.com/"));
        assert!(!allowed("https://facebook.com.evil.org/"));
        assert!(!allowed("https://example.com/"));
        assert!(!allowed("https://fbcdn.net.attacker.io/"));
    }

    #[test]
    fn blocks_credentials_and_nonstandard_ports() {
        assert!(!allowed("https://user:pass@www.facebook.com/"));
        assert!(!allowed("https://www.facebook.com:8443/"));
    }
}
