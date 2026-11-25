//! Rejestr modułów wyspy oraz wspólna logika WebView dla aplikacji webowych.

pub mod antigravity;
pub mod discord;
pub mod gemini;
pub mod messenger;
pub mod spotify;

use std::sync::Arc;

use island_core::is_allowed_url;
use serde::Serialize;
use tauri::webview::{NewWindowResponse, WebviewBuilder};
use tauri::{AppHandle, Manager, Runtime, WebviewUrl};
use tauri_plugin_opener::OpenerExt;
use url::Url;

/// Wspólny katalog danych WebView (sesje przeżywają restart aplikacji).
pub const PROFILE_DIR: &str = "webview-profile";

/// Flagi Chromium/WebView2 ograniczające liczbę procesów i zużycie RAM przy zachowaniu DRM (Widevine) i autoplay.
pub const BROWSER_ARGS: &str = "--renderer-process-limit=4 --process-per-site --js-flags=--max-old-space-size=256 --disk-cache-size=33554432 --disable-features=SpareRendererForSitePerProcess,BackForwardCache,Translate,OptimizationHints,CalculateNativeWinOcclusion,msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-sync --disable-default-apps --autoplay-policy=no-user-gesture-required";

const CHROME_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

/// Wymusza programowy moduł DRM Widevine zamiast PlayReady (który zawodzi w przezroczystych oknach warstwowych)
/// oraz powiadamia hosta o otwarciu/zamknięciu systemowego okna wyboru ekranu (`getDisplayMedia`).
const EME_INIT_SCRIPT: &str = r#"
(function() {
  if (navigator.requestMediaKeySystemAccess) {
    const orig = navigator.requestMediaKeySystemAccess.bind(navigator);
    navigator.requestMediaKeySystemAccess = function(keySystem, supportedConfigurations) {
      if (typeof keySystem === 'string' && keySystem.toLowerCase().includes('playready')) {
        return Promise.reject(new DOMException('PlayReady not supported in layered window', 'NotSupportedError'));
      }
      return orig(keySystem, supportedConfigurations);
    };
  }
  const postPicker = (msg) => {
    try {
      const wv = window.chrome && window.chrome.webview;
      if (wv && typeof wv.postMessage === 'function') {
        wv.postMessage(msg);
      }
    } catch {}
  };
  if (navigator.mediaDevices && typeof navigator.mediaDevices.getDisplayMedia === 'function') {
    const origDisplay = navigator.mediaDevices.getDisplayMedia.bind(navigator.mediaDevices);
    navigator.mediaDevices.getDisplayMedia = function(...args) {
      postPicker('__topisland:picker-open');
      try {
        const res = origDisplay(...args);
        if (res && typeof res.then === 'function') {
          return res.then(
            (stream) => { postPicker('__topisland:picker-close'); return stream; },
            (err) => { postPicker('__topisland:picker-close'); throw err; }
          );
        }
        postPicker('__topisland:picker-close');
        return res;
      } catch (e) {
        postPicker('__topisland:picker-close');
        throw e;
      }
    };
  }
  window.addEventListener('keydown', function(e) {
    if (e.key === 'Tab' && !e.ctrlKey && !e.altKey && !e.metaKey) {
      e.preventDefault();
      e.stopImmediatePropagation();
      postPicker(e.shiftKey ? '__topisland:cycle-prev' : '__topisland:cycle-next');
    }
  }, true);
})();
"#;

/// Lista wszystkich etykiet WebView obsługiwanych jako aplikacje wyspy.
pub const WEBVIEW_LABELS: &[&str] = &[
    messenger::LABEL,
    spotify::LABEL,
    discord::LABEL,
    gemini::LABEL,
];

/// Wspólny interfejs modułu. Moduły z natywnym WebView i moduły lekkie (HTML/JS, Rust)
/// implementują ten sam trait; rdzeń nie zna ich szczegółów.
pub trait IslandModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    /// Wyższy priorytet wygrywa w pigułce, gdy kilka modułów ma badge > 0.
    fn priority(&self) -> u8;
}

#[derive(Serialize, Clone, Debug)]
pub struct ModuleInfo {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: &'static str,
    pub priority: u8,
}

pub struct ModuleRegistry {
    modules: Vec<Arc<dyn IslandModule>>,
}

impl ModuleRegistry {
    pub fn new(modules: Vec<Arc<dyn IslandModule>>) -> Self {
        Self { modules }
    }

    pub fn list(&self) -> Vec<ModuleInfo> {
        self.modules
            .iter()
            .map(|m| ModuleInfo {
                id: m.id(),
                name: m.name(),
                icon: m.icon(),
                priority: m.priority(),
            })
            .collect()
    }

    pub fn get(&self, id: &str) -> Option<&Arc<dyn IslandModule>> {
        self.modules.iter().find(|m| m.id() == id)
    }
}

/// Zwraca docelowy adres URL startowy dla danego modułu webowego.
pub fn start_url(id: &str) -> Option<&'static str> {
    match id {
        messenger::LABEL => Some(messenger::START_URL),
        spotify::LABEL => Some(spotify::START_URL),
        discord::LABEL => Some(discord::START_URL),
        gemini::LABEL => Some(gemini::START_URL),
        _ => None,
    }
}

/// Zwraca statyczną etykietę modułu, jeśli `id` wskazuje na znany moduł wyspy.
pub fn static_label(id: &str) -> Option<&'static str> {
    if id == antigravity::LABEL {
        return Some(antigravity::LABEL);
    }
    WEBVIEW_LABELS.iter().copied().find(|&label| label == id)
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

/// Decyzja dla nawigacji najwyższego poziomu w WebView modułu.
fn handle_navigation<R: Runtime>(app: &AppHandle<R>, url: &Url) -> bool {
    if url.as_str() == "about:blank" || is_allowed_url(url) {
        return true;
    }
    open_external(app, url);
    false
}

/// Nowe okna (target=_blank / window.open): dozwolone domeny w tym samym WebView, reszta do przeglądarki.
fn handle_new_window<R: Runtime>(
    app: &AppHandle<R>,
    label: &'static str,
    url: Url,
) -> NewWindowResponse<R> {
    if is_allowed_url(&url) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            if let Some(webview) = app.get_webview(label) {
                if let Err(e) = webview.navigate(url) {
                    eprintln!("nawigacja nowego okna ({label}) nie powiodła się: {e}");
                }
            }
        });
    } else {
        open_external(app, &url);
    }
    NewWindowResponse::Deny
}

/// Konfiguruje WebView2: niski cel pamięci, wyłączenie zbędnych funkcji, zezwolenie na uprawnienia DRM/media
/// oraz nasłuchiwanie zdarzeń `getDisplayMedia`, skrótów Tab i pełnego ekranu (np. wideo/stream Discorda).
#[cfg(windows)]
pub fn configure_webview2_low_memory<R: Runtime>(app: &AppHandle<R>, webview: &tauri::Webview<R>) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
        COREWEBVIEW2_PERMISSION_STATE_ALLOW,
    };
    use webview2_com::{
        take_pwstr, ContainsFullScreenElementChangedEventHandler, PermissionRequestedEventHandler,
        WebMessageReceivedEventHandler,
    };
    use windows::core::{Interface, PWSTR};

    let allow_media_permissions = webview.label() != crate::core::window_manager::UI_LABEL;
    let app_for_msg = app.clone();
    let app_for_fs = app.clone();
    let label_for_fs = webview.label().to_string();

    let _ = webview.with_webview(move |platform| {
        let controller = platform.controller();
        unsafe {
            let Ok(core) = controller.CoreWebView2() else {
                return;
            };
            let mut browser_pid: u32 = 0;
            if core.BrowserProcessId(&mut browser_pid).is_ok() && browser_pid != 0 {
                crate::core::window_manager::register_webview_pid(browser_pid);
            }
            if let Ok(core19) = core.cast::<ICoreWebView2_19>() {
                let _ = core19.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
            }
            if let Ok(settings) = core.Settings() {
                let _ = settings.SetAreDevToolsEnabled(false);
                let _ = settings.SetIsStatusBarEnabled(false);
                let _ = settings.SetIsZoomControlEnabled(false);
                let _ = settings.SetIsBuiltInErrorPageEnabled(false);
                if allow_media_permissions {
                    let _ = settings.SetIsWebMessageEnabled(true);
                }
            }
            if allow_media_permissions {
                let mut token: i64 = 0;
                let _ = core.add_PermissionRequested(
                    &PermissionRequestedEventHandler::create(Box::new(|_, args| {
                        if let Some(args) = args {
                            let _ = args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW);
                        }
                        Ok(())
                    })),
                    &mut token,
                );

                let mut msg_token: i64 = 0;
                let _ = core.add_WebMessageReceived(
                    &WebMessageReceivedEventHandler::create(Box::new(move |_, args| {
                        if let Some(args) = args {
                            let mut raw = PWSTR::null();
                            if args.TryGetWebMessageAsString(&mut raw).is_ok() {
                                let msg = take_pwstr(raw);
                                match msg.trim_matches('"') {
                                    "__topisland:picker-open" => {
                                        crate::core::window_manager::set_dialog_open(
                                            &app_for_msg,
                                            true,
                                        );
                                    }
                                    "__topisland:picker-close" => {
                                        crate::core::window_manager::set_dialog_open(
                                            &app_for_msg,
                                            false,
                                        );
                                    }
                                    "__topisland:cycle-next" => {
                                        let _ = crate::core::window_manager::cycle_module(
                                            &app_for_msg,
                                            1,
                                        );
                                    }
                                    "__topisland:cycle-prev" => {
                                        let _ = crate::core::window_manager::cycle_module(
                                            &app_for_msg,
                                            -1,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Ok(())
                    })),
                    &mut msg_token,
                );

                let mut fs_token: i64 = 0;
                let _ = core.add_ContainsFullScreenElementChanged(
                    &ContainsFullScreenElementChangedEventHandler::create(Box::new(
                        move |sender, _args| {
                            if let Some(sender) = sender {
                                let mut is_fs = windows::core::BOOL(0);
                                if sender.ContainsFullScreenElement(&mut is_fs).is_ok() {
                                    crate::core::window_manager::set_webview_fullscreen(
                                        &app_for_fs,
                                        &label_for_fs,
                                        is_fs.as_bool(),
                                    );
                                }
                            }
                            Ok(())
                        },
                    )),
                    &mut fs_token,
                );
            }
        }
    });
}

#[cfg(not(windows))]
pub fn configure_webview2_low_memory<R: Runtime>(
    _app: &AppHandle<R>,
    _webview: &tauri::Webview<R>,
) {
}

/// Buduje WebView dla aplikacji webowej; domyślnie startuje wyłączone (`about:blank`).
pub fn webview_builder<R: Runtime>(
    app: &AppHandle<R>,
    label: &'static str,
) -> tauri::Result<WebviewBuilder<R>> {
    let profile = app.path().app_local_data_dir()?.join(PROFILE_DIR);
    let start = Url::parse("about:blank").expect("stały, poprawny URL");

    let nav_app = app.clone();
    let window_app = app.clone();

    Ok(WebviewBuilder::new(label, WebviewUrl::External(start))
        .data_directory(profile)
        .user_agent(CHROME_USER_AGENT)
        .additional_browser_args(BROWSER_ARGS)
        .initialization_script(EME_INIT_SCRIPT)
        .on_navigation(move |url| handle_navigation(&nav_app, url))
        .on_new_window(move |url, _features| handle_new_window(&window_app, label, url)))
}
