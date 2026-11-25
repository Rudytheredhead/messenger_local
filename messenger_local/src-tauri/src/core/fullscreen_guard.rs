//! Wykrywanie gier i filmów pełnoekranowych (Win32) i automatyczne ukrywanie wyspy.
//! Wątek działa co 500 ms; praca jest minimalna (kilka wywołań WinAPI).

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager, Runtime};

use crate::core::window_manager::{self, ISLAND_LABEL};

const POLL_INTERVAL: Duration = Duration::from_millis(500);

pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    thread::spawn(move || {
        let mut ticks: u32 = 0;
        loop {
            thread::sleep(POLL_INTERVAL);
            let own = own_hwnd(&app);
            if !window_manager::is_dialog_open(&app) && !window_manager::is_expanded(&app) {
                window_manager::set_fullscreen_hidden(&app, is_fullscreen_foreground(own));
            }
            ticks = ticks.wrapping_add(1);
            if ticks % 60 == 0 && !window_manager::is_expanded(&app) {
                crate::core::suspend::cleanup_working_sets();
            }
        }
    });
}

#[cfg(windows)]
fn own_hwnd<R: Runtime>(app: &AppHandle<R>) -> isize {
    app.get_window(ISLAND_LABEL)
        .and_then(|w| w.hwnd().ok())
        .map(|h| h.0 as isize)
        .unwrap_or(0)
}

#[cfg(not(windows))]
fn own_hwnd<R: Runtime>(_app: &AppHandle<R>) -> isize {
    0
}

#[cfg(windows)]
fn is_fullscreen_foreground(own_hwnd: isize) -> bool {
    use island_core::{covers, Rect};
    use windows::Win32::Foundation::RECT;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows::Win32::UI::Shell::{
        SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE,
        QUNS_RUNNING_D3D_FULL_SCREEN,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetWindowRect,
    };

    // 1. System sam zgłasza tryb pełnoekranowy / prezentację / grę D3D.
    if let Ok(state) = unsafe { SHQueryUserNotificationState() } {
        if state == QUNS_BUSY
            || state == QUNS_RUNNING_D3D_FULL_SCREEN
            || state == QUNS_PRESENTATION_MODE
        {
            return true;
        }
    }

    // 2. Aktywne okno pokrywa cały monitor (borderless / przeglądarka na pełnym ekranie).
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() || fg.0 as isize == own_hwnd {
            return false;
        }

        // Pulpit i pasek zadań zawsze pokrywają monitor — to nie jest tryb pełnoekranowy.
        let mut class_buf = [0u16; 64];
        let len = GetClassNameW(fg, &mut class_buf);
        let class = String::from_utf16_lossy(&class_buf[..len.max(0) as usize]);
        if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
            return false;
        }

        let mut win_rect = RECT::default();
        if GetWindowRect(fg, &mut win_rect).is_err() {
            return false;
        }

        let monitor = MonitorFromWindow(fg, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
        let mon = info.rcMonitor;

        covers(
            Rect { left: win_rect.left, top: win_rect.top, right: win_rect.right, bottom: win_rect.bottom },
            Rect { left: mon.left, top: mon.top, right: mon.right, bottom: mon.bottom },
        )
    }
}

#[cfg(not(windows))]
fn is_fullscreen_foreground(_own_hwnd: isize) -> bool {
    false
}
