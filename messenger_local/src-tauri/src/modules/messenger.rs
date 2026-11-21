//! Moduł Messenger: izolowany WebView z oficjalną stroną `https://www.facebook.com/messages`.
//! Licznik nieprzeczytanych czytany wyłącznie z tytułu karty (np. „(2) Messenger”) — bez DOM, bez API.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use island_core::{is_allowed_url, unread_from_title};
use tauri::webview::{NewWindowResponse, WebviewBuilder};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl};
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::core::window_manager;
use crate::modules::IslandModule;

pub const LABEL: &str = "messenger";
pub const START_URL: &str = "https://www.facebook.com/messages";
/// Wspólny katalog danych WebView (sesja Messengera przeżywa restart).
pub const PROFILE_DIR: &str = "webview-profile";

pub struct Messenger {
    unread: AtomicU32,
}

impl Messenger {
    pub fn new() -> Self {
        Self { unread: AtomicU32::new(0) }
    }

    pub fn set_unread(&self, count: u32) {
        self.unread.store(count, Ordering::Relaxed);
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
    fn badge(&self) -> u32 {
        self.unread.load(Ordering::Relaxed)
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

/// Builder WebView Messengera (dodawany jako dziecko okna wyspy).
pub fn builder<R: Runtime>(
    app: &AppHandle<R>,
    module: Arc<Messenger>,
) -> tauri::Result<WebviewBuilder<R>> {
    let profile = app.path().app_local_data_dir()?.join(PROFILE_DIR);
    let start = Url::parse(START_URL).expect("stały, poprawny URL");

    let nav_app = app.clone();
    let window_app = app.clone();
    let title_app = app.clone();

    Ok(WebviewBuilder::new(LABEL, WebviewUrl::External(start))
        .data_directory(profile)
        .on_navigation(move |url| handle_navigation(&nav_app, url))
        .on_new_window(move |url, _features| handle_new_window(&window_app, url))
        .on_document_title_changed(move |_webview, title| {
            module.set_unread(unread_from_title(&title));
            window_manager::emit_state(&title_app);
        }))
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
