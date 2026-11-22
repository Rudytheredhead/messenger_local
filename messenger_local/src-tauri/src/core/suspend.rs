//! Natychmiastowe uśpianie WebView Messengera przy zwinięciu wyspy.
//!
//! Po zwinięciu wyspy:
//! 1. Ukrywa WebView messenger.
//! 2. Nawiguje na about:blank (zniszczy DOM, V8 heap, WebSockety).
//! 3. Po krótkim opóźnieniu (200 ms) wywołuje TrySuspend + czyści Working Set.
//! Przy rozwinięciu: Resume + nawigacja na START_URL + pokazanie WebView.

use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};
use windows::core::PCWSTR;

use crate::modules::messenger;

/// Opóźnienie przed TrySuspend/WorkingSet cleanup (ms) – daje czas na about:blank nawigację.
const SUSPEND_DELAY_MS: u64 = 200;

/// Stan uśpienia – generation chroni przed uśpieniem po ponownym rozwinięciu.
#[derive(Default)]
pub struct SuspendState {
    generation: AtomicU64,
}

impl SuspendState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Wywoływane przy każdym zwinięciu (i na starcie, bo wyspa startuje zwinięta).
    pub fn schedule<R: Runtime>(&self, app: &AppHandle<R>) {
        let r#gen = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let app = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(SUSPEND_DELAY_MS));
            // Sprawdź, czy w międzyczasie nie nastąpiło rozwinięcie (nowe generation).
            let current = app.state::<SuspendState>().generation.load(Ordering::Relaxed);
            if current == r#gen && !is_expanded(&app) {
                set_suspended(&app, true);
            }
        });
    }

    /// Wywoływane przy rozwinięciu – zawsze wznawia.
    pub fn resume<R: Runtime>(&self, app: &AppHandle<R>) {
        self.generation.fetch_add(1, Ordering::Relaxed);
        set_suspended(app, false);
    }
}

fn is_expanded<R: Runtime>(app: &AppHandle<R>) -> bool {
    use crate::core::state::IslandStore;
    let store = app.state::<IslandStore>();
    let guard = store.0.lock().unwrap_or_else(|e| e.into_inner());
    guard.expanded
}

#[cfg(windows)]
fn set_suspended<R: Runtime>(app: &AppHandle<R>, suspend: bool) {
    use tauri::Manager as _;
    use webview2_com::Microsoft::Web::WebView2::Win32::{ICoreWebView2_3, ICoreWebView2_19, COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW};
    use webview2_com::TrySuspendCompletedHandler;
    use windows::core::Interface;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Memory::SetProcessWorkingSetSizeEx;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA};
    use windows::Win32::System::Memory::SETPROCESSWORKINGSETSIZEEX_FLAGS;

    let Some(webview) = app.get_webview(messenger::LABEL) else {
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

                // 4. Po opóźnieniu (już w wątku z schedule) – TrySuspend + WorkingSet cleanup.
                // TrySuspend
                if let Ok(core3) = core.cast::<ICoreWebView2_3>() {
                    let handler = TrySuspendCompletedHandler::create(Box::new(|_hr, _ok| Ok(())));
                    let _ = core3.TrySuspend(&handler);
                }

                // 5. Wyczyść Working Set: bieżący proces + procesy potomne msedgewebview2.exe.
                cleanup_working_sets();
            } else {
                // Resume – wznawiamy, nawigacja na START_URL robi window_manager.
                if let Ok(core3) = core.cast::<ICoreWebView2_3>() {
                    let _ = core3.Resume();
                }
            }
        }
    });
}

#[cfg(windows)]
fn cleanup_working_sets() {
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Memory::SetProcessWorkingSetSizeEx;
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_QUOTA};
    use windows::Win32::System::Memory::SETPROCESSWORKINGSETSIZEEX_FLAGS;

    unsafe {
        // Bieżący proces - ustaw min/max working set by go wyczyścić
        let current = GetCurrentProcess();
        let _ = SetProcessWorkingSetSizeEx(current, usize::MIN, usize::MAX, SETPROCESSWORKINGSETSIZEEX_FLAGS::default());

        // Procesy potomne msedgewebview2.exe
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
                    if let Ok(h) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA, false, entry.th32ProcessID) {
                        let _ = SetProcessWorkingSetSizeEx(h, usize::MIN, usize::MAX, SETPROCESSWORKINGSETSIZEEX_FLAGS::default());
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
fn set_suspended<R: Runtime>(_app: &AppHandle<R>, _suspend: bool) {}
