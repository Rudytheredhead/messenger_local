//! Uśpianie wyłączonych WebView aplikacji (about:blank + TrySuspend + Working Set cleanup).

use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};
use windows::core::PCWSTR;

use crate::modules;

/// Opóźnienie przed TrySuspend/WorkingSet cleanup (ms) – daje czas na about:blank nawigację.
const SUSPEND_DELAY_MS: u64 = 200;

/// Stan uśpienia per WebView – generation chroni przed uśpieniem po ponownym włączeniu modułu.
#[derive(Default)]
pub struct SuspendState {
    generations: Mutex<HashMap<String, u64>>,
}

impl SuspendState {
    pub fn new() -> Self {
        Self::default()
    }

    fn bump_generation(&self, label: &str) -> u64 {
        let mut map = self.generations.lock().unwrap_or_else(|e| e.into_inner());
        let next = map.get(label).copied().unwrap_or(0) + 1;
        map.insert(label.to_string(), next);
        next
    }

    fn current_generation(&self, label: &str) -> u64 {
        let map = self.generations.lock().unwrap_or_else(|e| e.into_inner());
        map.get(label).copied().unwrap_or(0)
    }

    /// Planuje uśpienie wskazanego WebView po zamknięciu modułu (lub na starcie).
    pub fn schedule_webview<R: Runtime>(&self, app: &AppHandle<R>, label: &str) {
        let r#gen = self.bump_generation(label);
        let app = app.clone();
        let label = label.to_string();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(SUSPEND_DELAY_MS));
            let current = app.state::<SuspendState>().current_generation(&label);
            if current == r#gen && !is_module_running(&app, &label) {
                set_suspended(&app, &label, true);
            }
        });
    }

    /// Planuje uśpienie wszystkich domyślnie wyłączonych WebView na starcie aplikacji.
    pub fn schedule_all<R: Runtime>(&self, app: &AppHandle<R>) {
        for &label in modules::WEBVIEW_LABELS {
            self.schedule_webview(app, label);
        }
    }

    /// Wznawia wskazany WebView przy włączaniu aplikacji przez użytkownika.
    pub fn resume_webview<R: Runtime>(&self, app: &AppHandle<R>, label: &str) {
        self.bump_generation(label);
        set_suspended(app, label, false);
    }
}

fn is_module_running<R: Runtime>(app: &AppHandle<R>, label: &str) -> bool {
    use crate::core::state::IslandStore;
    let store = app.state::<IslandStore>();
    let guard = store.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.is_module_running(label)
}

#[cfg(windows)]
fn set_suspended<R: Runtime>(app: &AppHandle<R>, label: &str, suspend: bool) {
    use tauri::Manager as _;
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_19, ICoreWebView2_3, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    };
    use webview2_com::TrySuspendCompletedHandler;
    use windows::core::Interface;

    let Some(webview) = app.get_webview(label) else {
        return;
    };

    let _ = webview.with_webview(move |platform| {
        let controller = platform.controller();
        unsafe {
            let Ok(core) = controller.CoreWebView2() else {
                return;
            };

            if suspend {
                // 1. Nawiguj na about:blank (zniszczy DOM, V8 heap, WebSockety).
                let blank_wide: Vec<u16> = "about:blank".encode_utf16().chain(Some(0)).collect();
                let _ = core.Navigate(PCWSTR(blank_wide.as_ptr()));

                // 2. Ustaw niski cel zużycia pamięci (ICoreWebView2_19).
                if let Ok(core19) = core.cast::<ICoreWebView2_19>() {
                    let _ = core19.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW);
                }

                // 3. Wyłącz zbędne ustawienia WebView2.
                if let Ok(settings) = core.Settings() {
                    let _ = settings.SetAreDevToolsEnabled(false);
                    let _ = settings.SetIsStatusBarEnabled(false);
                    let _ = settings.SetIsZoomControlEnabled(false);
                    let _ = settings.SetIsBuiltInErrorPageEnabled(false);
                }

                // 4. TrySuspend
                if let Ok(core3) = core.cast::<ICoreWebView2_3>() {
                    let handler = TrySuspendCompletedHandler::create(Box::new(|_hr, _ok| Ok(())));
                    let _ = core3.TrySuspend(&handler);
                }

                // 5. Wyczyść Working Set: bieżący proces + procesy potomne msedgewebview2.exe.
                cleanup_working_sets();
            } else {
                // Resume – wznawiamy, nawigację na START_URL robi window_manager.
                if let Ok(core3) = core.cast::<ICoreWebView2_3>() {
                    let _ = core3.Resume();
                }
            }
        }
    });
}

#[cfg(windows)]
pub fn cleanup_working_sets() {
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Memory::{
        SetProcessWorkingSetSizeEx, SETPROCESSWORKINGSETSIZEEX_FLAGS,
    };
    use windows::Win32::System::Threading::{
        GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA,
    };

    unsafe {
        let current = GetCurrentProcess();
        let _ = SetProcessWorkingSetSizeEx(
            current,
            usize::MAX,
            usize::MAX,
            SETPROCESSWORKINGSETSIZEEX_FLAGS::default(),
        );

        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return;
        };
        if snapshot.is_invalid() {
            return;
        }
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile);
                if name.to_lowercase().contains("msedgewebview2") {
                    if let Ok(h) = OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA,
                        false,
                        entry.th32ProcessID,
                    ) {
                        let _ = SetProcessWorkingSetSizeEx(
                            h,
                            usize::MAX,
                            usize::MAX,
                            SETPROCESSWORKINGSETSIZEEX_FLAGS::default(),
                        );
                        let _ = windows::Win32::Foundation::CloseHandle(h);
                    }
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
    }
}

#[cfg(not(windows))]
pub fn cleanup_working_sets() {}

/// Opróżnia Working Set bieżącego procesu i procesów WebView2 po zadanym opóźnieniu.
pub fn trim_working_sets_delayed(delay_ms: u64) {
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(delay_ms));
        cleanup_working_sets();
    });
}

#[cfg(not(windows))]
fn set_suspended<R: Runtime>(_app: &AppHandle<R>, _label: &str, _suspend: bool) {}
