//! Moduł Messenger: izolowany WebView z oficjalną stroną `https://www.facebook.com/messages`.

use std::sync::Arc;

use island_core::is_allowed_url;
use tauri::webview::{NewWindowResponse, WebviewBuilder};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::modules::IslandModule;

pub const LABEL: &str = "messenger";
pub const START_URL: &str = "https://www.facebook.com/messages";
/// Wspólny katalog danych WebView (sesja Messengera przeżywa restart).
pub const PROFILE_DIR: &str = "webview-profile";

pub struct Messenger;

impl Messenger {
    pub fn new() -> Self {
        Self
    }
}

impl IslandModule for Messenger {
    fn id(&self) -> &'static str {
        "messenger"
    }
    fn name(&self) -> &'static str {
        "Messenger"
    }
    fn icon(&self) -> &'static str {
        "💬"
    }
    fn priority(&self) -> u8 {
        100
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

/// Decyzja dla nawigacji najwyższego poziomu w WebView Messengera.
fn handle_navigation<R: Runtime>(app: &AppHandle<R>, url: &Url) -> bool {
    if url.as_str() == "about:blank" || is_allowed_url(url) {
        return true;
    }
    open_external(app, url);
    false
}

/// Nowe okna (target=_blank / window.open): domeny Mety w tym samym WebView, reszta do przeglądarki.
fn handle_new_window<R: Runtime>(app: &AppHandle<R>, url: Url) -> NewWindowResponse<R> {
    if is_allowed_url(&url) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Some(webview) = app.get_webview(LABEL) {
                if let Err(e) = webview.navigate(url) {
                    eprintln!("nawigacja nowego okna nie powiodła się: {e}");
                }
            }
        });
    } else {
        open_external(app, &url);
    }
    NewWindowResponse::Deny
}

/// Konfiguruje WebView2: niski cel pamięci + wyłączenie zbędnych funkcji.
#[cfg(windows)]
pub fn configure_webview2_low_memory<R: Runtime>(webview: &tauri::Webview<R>) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW};
    use windows::core::Interface;

    let _ = webview.with_webview(move |platform| {
        let controller = platform.controller();
        unsafe {
            let Ok(core) = controller.CoreWebView2() else {
                return;
            };
            // Niski cel zużycia pamięci (ICoreWebView2_19).
            if let Ok(core19) = core.cast::<ICoreWebView2_19>() {
                let _ = core19.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
            }
            // Wyłącz zbędne ustawienia.
            if let Ok(settings) = core.Settings() {
                let _ = settings.SetAreDevToolsEnabled(false);
                let _ = settings.SetIsStatusBarEnabled(false);
                let _ = settings.SetIsZoomControlEnabled(false);
                let _ = settings.SetIsBuiltInErrorPageEnabled(false);
            }
        }
    });
}

#[cfg(not(windows))]
fn configure_webview2_low_memory<R: Runtime>(_webview: &tauri::Webview<R>) {}

/// Builder WebView Messengera (dodawany jako dziecko okna wyspy).
pub fn builder<R: Runtime>(
    app: &AppHandle<R>,
    _module: Arc<Messenger>,
) -> tauri::Result<WebviewBuilder<R>> {
    let profile = app.path().app_local_data_dir()?.join(PROFILE_DIR);
    let start = Url::parse(START_URL).expect("stały, poprawny URL");

    let nav_app = app.clone();
    let window_app = app.clone();

    let builder = WebviewBuilder::new(LABEL, WebviewUrl::External(start))
        .data_directory(profile)
        .on_navigation(move |url| handle_navigation(&nav_app, url))
        .on_new_window(move |url, _features| handle_new_window(&window_app, url));

    // Ustaw niski profil pamięci i wyłącz zbędne funkcje po utworzeniu WebView.
    // Używamy on_did_create_context lub with_webview po dodaniu do okna.
    // Tutaj zwracamy builder, a konfigurację robimy w window_manager::create po add_child.
    Ok(builder)
}

/// Usuwa ciasteczka, cache i pamięć lokalną WebView Messengera, potem wraca na stronę logowania.
pub fn logout<R: Runtime>(app: &AppHandle<R>) {
    if let Some(webview) = app.get_webview(LABEL) {
        if let Err(e) = webview.clear_all_browsing_data() {
            eprintln!("nie udało się wyczyścić danych przeglądarki: {e}");
        }
        if let Ok(url) = Url::parse(START_URL) {
            let _ = webview.navigate(url);
        }
    }
}