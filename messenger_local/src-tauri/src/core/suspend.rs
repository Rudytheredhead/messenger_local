//! Usypianie WebView Messengera przy zwiniętej wyspie (WebView2 TrySuspend / Resume).
//!
//! Uwaga: uśpiona strona nie odbiera wiadomości, więc licznik nieprzeczytanych jest zamrożony.
//! Dlatego funkcja jest OPCJONALNA i domyślnie wyłączona (przełącznik w zasobniku).

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};

use crate::core::window_manager;
use crate::modules::messenger;

const DEFAULT_DELAY_SECS: u64 = 300;

pub struct SuspendPolicy {
    enabled: AtomicBool,
    /// Zwiększane przy każdym zwinięciu/rozwinięciu — stary timer nie uśpi strony po ponownym rozwinięciu.
    generation: AtomicU64,
    delay: Duration,
}

impl SuspendPolicy {
    /// Stan początkowy z ustawień środowiska (do testów i strojenia):
    /// `MESSENGER_ISLAND_AUTO_SUSPEND=1`, `MESSENGER_ISLAND_SUSPEND_SECS=<n>`.
    pub fn from_env() -> Self {
        let enabled = std::env::var("MESSENGER_ISLAND_AUTO_SUSPEND").is_ok_and(|v| v == "1");
        let secs = std::env::var("MESSENGER_ISLAND_SUSPEND_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_DELAY_SECS);
        Self {
            enabled: AtomicBool::new(enabled),
            generation: AtomicU64::new(0),
            delay: Duration::from_secs(secs),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
}

/// Wywoływane przy każdym zwinięciu (i na starcie, bo wyspa startuje zwinięta).
pub fn schedule<R: Runtime>(app: &AppHandle<R>) {
    let policy = app.state::<SuspendPolicy>();
    let generation = policy.generation.fetch_add(1, Ordering::Relaxed) + 1;
    if !policy.enabled() {
        return;
    }
    let delay = policy.delay;
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(delay);
        let policy = app.state::<SuspendPolicy>();
        let still_current = policy.generation.load(Ordering::Relaxed) == generation;
        if still_current && policy.enabled() && !window_manager::is_expanded(&app) {
            set_suspended(&app, true);
        }
    });
}

/// Wywoływane przy rozwinięciu — zawsze, bo Resume na aktywnej stronie jest nieszkodliwe.
pub fn resume<R: Runtime>(app: &AppHandle<R>) {
    app.state::<SuspendPolicy>().generation.fetch_add(1, Ordering::Relaxed);
    set_suspended(app, false);
}

/// Przełącznik z zasobnika. Wyłączenie od razu wznawia stronę.
pub fn set_enabled<R: Runtime>(app: &AppHandle<R>, enabled: bool) {
    app.state::<SuspendPolicy>().set_enabled(enabled);
    if enabled {
        schedule(app);
    } else {
        set_suspended(app, false);
    }
}

#[cfg(windows)]
fn set_suspended<R: Runtime>(app: &AppHandle<R>, suspend: bool) {
    use tauri::Manager as _;
    use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2_3;
    use webview2_com::TrySuspendCompletedHandler;
    use windows::core::Interface;

    let Some(webview) = app.get_webview(messenger::LABEL) else {
        return;
    };
    // with_webview wykonuje się na wątku UI; WebView2 wymaga tam wywołań COM.
    let _ = webview.with_webview(move |platform| {
        let controller = platform.controller();
        unsafe {
            let Ok(core) = controller.CoreWebView2() else {
                return;
            };
            let Ok(core3) = core.cast::<ICoreWebView2_3>() else {
                return;
            };
            if suspend {
                let handler = TrySuspendCompletedHandler::create(Box::new(|_hr, _ok| Ok(())));
                let _ = core3.TrySuspend(&handler);
            } else {
                let _ = core3.Resume();
            }
        }
    });
}

#[cfg(not(windows))]
fn set_suspended<R: Runtime>(_app: &AppHandle<R>, _suspend: bool) {}
