use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Duration;

use island_core::{
    centered_x, ease_out, lerp, BAR_HEIGHT, COLLAPSED_SIZE, EXPANDED_SIZE, EXPANDED_TOP_MARGIN,
    LARGE_SIZE, TOP_MARGIN,
};
use serde::Serialize;
use tauri::webview::WebviewBuilder;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime, Webview, WebviewUrl,
    Window, WindowBuilder,
};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::core::state::{IslandFlags, IslandStore};
use crate::core::suspend::{self, SuspendState};
use crate::modules::{self, antigravity::AntigravitySession};

pub const ISLAND_LABEL: &str = "island";
pub const UI_LABEL: &str = "island-ui";
const UI_PAGE: &str = "index.html";
/// Przezroczyste tło WebView (domyślnie biały prostokąt wokół pigułki).
const TRANSPARENT: tauri::window::Color = tauri::window::Color(0, 0, 0, 0);
pub const STATE_EVENT: &str = "island://state";

const ANIM_STEPS: u32 = 10;
/// Promienie rogów — zgodne z CSS (`styles.css`).
const COLLAPSED_RADIUS: f64 = 10.0;
const EXPANDED_RADIUS: f64 = 22.0;
const ANIM_FRAME: Duration = Duration::from_millis(16);

#[derive(Serialize, Clone, Debug)]
pub struct IslandState {
    pub expanded: bool,
    pub large: bool,
    pub user_hidden: bool,
    pub fullscreen_hidden: bool,
    pub messenger_running: bool,
    pub spotify_running: bool,
    pub discord_running: bool,
    pub gemini_running: bool,
    pub antigravity_running: bool,
    pub active_module: String,
}

#[cfg(windows)]
fn clear_window_corners(hwnd: isize) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::SetWindowRgn;
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = SetWindowRgn(HWND(hwnd as *mut std::ffi::c_void), None, true);
    }
}

#[cfg(not(windows))]
fn clear_window_corners(_hwnd: isize) {}

/// Zaokrągla rogi okna regionem Win32. Region obcina też child WebView-y aplikacji,
/// więc CSS nie wystarcza. Gdy `round_top` jest `false` (zwinięty pasek), górna krawędź jest
/// rozciągnięta poza okno, aby tylko dolne rogi były zaokrąglone; gdy `true`, zaokrąglone są wszystkie 4 rogi.
#[cfg(windows)]
fn round_window_corners(
    hwnd: isize,
    scale: f64,
    width: f64,
    height: f64,
    radius: f64,
    round_top: bool,
) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
    if hwnd == 0 {
        return;
    }
    let r = (radius * scale).round() as i32;
    let w = (width * scale).round() as i32;
    let h = (height * scale).round() as i32;
    let top = if round_top { 0 } else { -r };
    unsafe {
        let region = CreateRoundRectRgn(0, top, w, h, 2 * r, 2 * r);
        // System przejmuje region po udanym SetWindowRgn — nie zwalniamy go ręcznie.
        SetWindowRgn(HWND(hwnd as *mut std::ffi::c_void), Some(region), true);
    }
}

#[cfg(not(windows))]
fn round_window_corners(
    _hwnd: isize,
    _scale: f64,
    _width: f64,
    _height: f64,
    _radius: f64,
    _round_top: bool,
) {
}

pub fn snapshot<R: Runtime>(app: &AppHandle<R>) -> IslandState {
    let flags = current_flags(app);
    IslandState {
        expanded: flags.expanded,
        large: flags.large,
        user_hidden: flags.user_hidden,
        fullscreen_hidden: flags.fullscreen_hidden,
        messenger_running: flags.messenger_running,
        spotify_running: flags.spotify_running,
        discord_running: flags.discord_running,
        gemini_running: flags.gemini_running,
        antigravity_running: flags.antigravity_running,
        active_module: flags.active_module.to_string(),
    }
}

pub fn emit_state<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(STATE_EVENT, snapshot(app));
}

pub fn is_expanded<R: Runtime>(app: &AppHandle<R>) -> bool {
    current_flags(app).expanded
}

pub fn is_dialog_open<R: Runtime>(app: &AppHandle<R>) -> bool {
    current_flags(app).dialog_open
}

static WEBVIEW_PIDS: OnceLock<Mutex<HashSet<u32>>> = OnceLock::new();
static TRANSIENT_DIALOG_WATCHING: AtomicBool = AtomicBool::new(false);

pub fn register_webview_pid(pid: u32) {
    if pid == 0 {
        return;
    }
    let set = WEBVIEW_PIDS.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = set.lock().unwrap_or_else(|e| e.into_inner());
    guard.insert(pid);
}

fn register_escape_shortcut<R: Runtime>(app: &AppHandle<R>) {
    let handle = app.clone();
    let _ = app
        .global_shortcut()
        .on_shortcut("escape", move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Pressed && !is_dialog_open(&handle) {
                let _ = set_expanded(&handle, false);
            }
        });
}

fn unregister_escape_shortcut<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.global_shortcut().unregister("escape");
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
fn is_webview_or_child_pid(fg_pid: u32) -> bool {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;

    if fg_pid == 0 {
        return false;
    }
    let self_pid = unsafe { GetCurrentProcessId() };
    if fg_pid == self_pid {
        return true;
    }
    let pids: HashSet<u32> = WEBVIEW_PIDS
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    if pids.contains(&fg_pid) {
        return true;
    }

    let Ok(snapshot) = (unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }) else {
        return false;
    };
    if snapshot.is_invalid() {
        return false;
    }

    let mut parents: HashMap<u32, u32> = HashMap::new();
    let mut entry = PROCESSENTRY32W {
        dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
        ..Default::default()
    };
    unsafe {
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }

    let mut cur = fg_pid;
    let mut visited = HashSet::new();
    for _ in 0..16 {
        let Some(&parent) = parents.get(&cur) else {
            break;
        };
        if parent == 0 || !visited.insert(parent) {
            break;
        }
        if parent == self_pid || pids.contains(&parent) {
            return true;
        }
        cur = parent;
    }
    false
}

#[cfg(windows)]
fn is_foreground_internal_or_webview_dialog(own_hwnd: isize) -> bool {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindow, GetWindowThreadProcessId, IsChild, GW_OWNER,
    };

    if own_hwnd == 0 {
        return false;
    }
    let own = HWND(own_hwnd as *mut std::ffi::c_void);
    unsafe {
        let fg = GetForegroundWindow();
        if fg.0.is_null() {
            return false;
        }
        if fg.0 as isize == own_hwnd || IsChild(own, fg).as_bool() {
            return true;
        }
        let mut cur = fg;
        for _ in 0..8 {
            let Ok(owner) = GetWindow(cur, GW_OWNER) else {
                break;
            };
            if owner.0.is_null() {
                break;
            }
            if owner.0 as isize == own_hwnd {
                return true;
            }
            cur = owner;
        }

        let mut fg_pid: u32 = 0;
        let _ = GetWindowThreadProcessId(fg, Some(&mut fg_pid));
        is_webview_or_child_pid(fg_pid)
    }
}

#[cfg(not(windows))]
fn is_foreground_internal_or_webview_dialog(_own_hwnd: isize) -> bool {
    false
}

pub fn set_dialog_open<R: Runtime>(app: &AppHandle<R>, open: bool) {
    update_flags(app, |f| f.dialog_open = open);
    if open {
        unregister_escape_shortcut(app);
        if let Some(window) = app.get_window(ISLAND_LABEL) {
            let _ = window.set_always_on_top(false);
        }
    } else {
        let flags = current_flags(app);
        if let Some(window) = app.get_window(ISLAND_LABEL) {
            if !flags.user_hidden && !flags.fullscreen_hidden {
                let _ = window.set_always_on_top(true);
            }
            let _ = window.set_focus();
        }
        if flags.expanded {
            register_escape_shortcut(app);
        }
    }
}

/// Obsługuje utratę fokusu głównego okna wyspy: nie zwija wyspy, jeśli fokus przejęło okno dialogowe WebView2 (np. picker ekranu Discorda).
pub fn handle_focus_lost<R: Runtime>(app: &AppHandle<R>) {
    let flags = current_flags(app);
    if !flags.expanded || flags.dialog_open || flags.webview_fullscreen {
        return;
    }
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(50));
        let flags = current_flags(&app);
        if !flags.expanded || flags.dialog_open || flags.webview_fullscreen {
            return;
        }
        if app
            .get_window(ISLAND_LABEL)
            .and_then(|w| w.is_focused().ok())
            .unwrap_or(false)
        {
            return;
        }

        let own = own_hwnd(&app);
        if !is_foreground_internal_or_webview_dialog(own) {
            let _ = set_expanded(&app, false);
            return;
        }

        if TRANSIENT_DIALOG_WATCHING.swap(true, Ordering::SeqCst) {
            return;
        }
        unregister_escape_shortcut(&app);
        if let Some(window) = app.get_window(ISLAND_LABEL) {
            let _ = window.set_always_on_top(false);
        }

        while is_expanded(&app)
            && (is_dialog_open(&app) || is_foreground_internal_or_webview_dialog(own))
        {
            thread::sleep(Duration::from_millis(150));
        }

        TRANSIENT_DIALOG_WATCHING.store(false, Ordering::SeqCst);
        if !is_expanded(&app) {
            return;
        }

        let flags = current_flags(&app);
        if let Some(window) = app.get_window(ISLAND_LABEL) {
            if !flags.user_hidden && !flags.fullscreen_hidden {
                let _ = window.set_always_on_top(true);
            }
        }

        thread::sleep(Duration::from_millis(200));
        if !is_expanded(&app) || is_dialog_open(&app) {
            return;
        }
        let still_internal = app
            .get_window(ISLAND_LABEL)
            .and_then(|w| w.is_focused().ok())
            .unwrap_or(false)
            || is_foreground_internal_or_webview_dialog(own);
        if still_internal {
            register_escape_shortcut(&app);
        } else {
            let _ = set_expanded(&app, false);
        }
    });
}

fn current_flags<R: Runtime>(app: &AppHandle<R>) -> IslandFlags {
    let store = app.state::<IslandStore>();
    let guard = store.0.lock().unwrap_or_else(|e| e.into_inner());
    *guard
}

/// Zmienia flagi; zwraca true, jeśli coś się faktycznie zmieniło.
fn update_flags<R: Runtime>(app: &AppHandle<R>, f: impl FnOnce(&mut IslandFlags)) -> bool {
    let store = app.state::<IslandStore>();
    let mut guard = store.0.lock().unwrap_or_else(|e| e.into_inner());
    let before = *guard;
    f(&mut guard);
    before != *guard
}

fn active_expanded_size(flags: IslandFlags) -> (f64, f64) {
    if flags.large {
        LARGE_SIZE
    } else {
        EXPANDED_SIZE
    }
}

fn show_webview<R: Runtime>(wv: &Webview<R>, size: (f64, f64)) {
    let _ = wv.set_position(LogicalPosition::new(0.0, BAR_HEIGHT));
    let _ = wv.set_size(LogicalSize::new(size.0, size.1 - BAR_HEIGHT));
    let _ = wv.show();
}

fn hide_all_module_webviews<R: Runtime>(app: &AppHandle<R>) {
    for &label in modules::WEBVIEW_LABELS {
        if let Some(wv) = app.get_webview(label) {
            let _ = wv.hide();
        }
    }
}

fn hide_other_module_webviews<R: Runtime>(app: &AppHandle<R>, active_label: &str) {
    for &label in modules::WEBVIEW_LABELS {
        if label != active_label {
            if let Some(wv) = app.get_webview(label) {
                let _ = wv.hide();
            }
        }
    }
}

/// Tworzy okno wyspy, WebView interfejsu (`island-ui`) oraz child WebView dla wszystkich aplikacji
/// (Messenger, Spotify, Discord) na `about:blank`, w stanie uśpienia do czasu włączenia.
pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let (w, h) = COLLAPSED_SIZE;
    let window = WindowBuilder::new(app, ISLAND_LABEL)
        .title("TopIsland")
        .inner_size(w, h)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .content_protected(true)
        .build()?;

    let profile = app.path().app_local_data_dir()?.join(modules::PROFILE_DIR);
    let ui = WebviewBuilder::new(UI_LABEL, WebviewUrl::App(UI_PAGE.into()))
        .transparent(true)
        .background_color(TRANSPARENT)
        .data_directory(profile)
        .additional_browser_args(modules::BROWSER_ARGS);
    window.add_child(ui, LogicalPosition::new(0.0, 0.0), LogicalSize::new(w, h))?;

    if let Some(ui_wv) = app.get_webview(UI_LABEL) {
        modules::configure_webview2_low_memory(app, &ui_wv);
    }

    let (ew, eh) = active_expanded_size(current_flags(app));
    for &label in modules::WEBVIEW_LABELS {
        window.add_child(
            modules::webview_builder(app, label)?,
            LogicalPosition::new(0.0, BAR_HEIGHT),
            LogicalSize::new(ew, eh - BAR_HEIGHT),
        )?;
        if let Some(wv) = app.get_webview(label) {
            modules::configure_webview2_low_memory(app, &wv);
            wv.hide()?;
        }
    }

    app.state::<SuspendState>().schedule_all(app);
    suspend::trim_working_sets_delayed(1000);

    let (origin_x, screen_w) = monitor_metrics(&window)?;
    window.set_position(LogicalPosition::new(
        origin_x + centered_x(screen_w, w),
        TOP_MARGIN,
    ))?;
    let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    round_window_corners(hwnd, window.scale_factor()?, w, h, COLLAPSED_RADIUS, false);
    Ok(())
}

/// (lewy margines monitora, szerokość monitora) w logicznych px; dla bieżącego monitora okna.
fn monitor_metrics<R: Runtime>(window: &Window<R>) -> tauri::Result<(f64, f64)> {
    let monitor = match window.current_monitor()? {
        Some(m) => Some(m),
        None => window.primary_monitor()?,
    };
    Ok(match monitor {
        Some(m) => {
            let scale = m.scale_factor();
            (m.position().x as f64 / scale, m.size().width as f64 / scale)
        }
        None => (0.0, 1920.0),
    })
}

/// (x, y, szerokość, wysokość) pełnego bieżącego monitora w logicznych px.
fn monitor_full_rect<R: Runtime>(window: &Window<R>) -> tauri::Result<(f64, f64, f64, f64)> {
    let monitor = match window.current_monitor()? {
        Some(m) => Some(m),
        None => window.primary_monitor()?,
    };
    Ok(match monitor {
        Some(m) => {
            let scale = m.scale_factor();
            (
                m.position().x as f64 / scale,
                m.position().y as f64 / scale,
                m.size().width as f64 / scale,
                m.size().height as f64 / scale,
            )
        }
        None => (0.0, 0.0, 1920.0, 1080.0),
    })
}

/// Przełącza okno wyspy i wybrany WebView w tryb pełnoekranowy (np. dla wideo/streamu Discorda).
pub fn set_webview_fullscreen<R: Runtime>(app: &AppHandle<R>, label: &str, fullscreen: bool) {
    let flags = current_flags(app);
    if fullscreen {
        if !flags.expanded || !flags.is_module_running(label) || flags.active_module != label {
            return;
        }
        if !update_flags(app, |f| f.webview_fullscreen = true) && flags.webview_fullscreen {
            return;
        }
        unregister_escape_shortcut(app);

        let Some(window) = app.get_window(ISLAND_LABEL) else {
            return;
        };
        let Ok((mx, my, mw, mh)) = monitor_full_rect(&window) else {
            return;
        };
        let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        clear_window_corners(hwnd);
        let _ = window.set_position(LogicalPosition::new(mx, my));
        let _ = window.set_size(LogicalSize::new(mw, mh));
        let _ = window.set_always_on_top(true);

        if let Some(ui) = app.get_webview(UI_LABEL) {
            let _ = ui.hide();
        }
        if let Some(wv) = app.get_webview(label) {
            let _ = wv.set_position(LogicalPosition::new(0.0, 0.0));
            let _ = wv.set_size(LogicalSize::new(mw, mh));
            let _ = wv.show();
            let _ = wv.set_focus();
        }
    } else {
        if !update_flags(app, |f| f.webview_fullscreen = false) {
            return;
        }
        let flags = current_flags(app);
        let (Some(window), Some(ui)) = (
            app.get_window(ISLAND_LABEL),
            app.get_webview(UI_LABEL),
        ) else {
            return;
        };
        if flags.expanded {
            let (ew, eh) = active_expanded_size(flags);
            if let Ok((origin_x, screen_w)) = monitor_metrics(&window) {
                let _ = window.set_position(LogicalPosition::new(
                    origin_x + centered_x(screen_w, ew),
                    EXPANDED_TOP_MARGIN,
                ));
            }
            let _ = window.set_size(LogicalSize::new(ew, eh));
            if let Ok(scale) = window.scale_factor() {
                let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
                round_window_corners(hwnd, scale, ew, eh, EXPANDED_RADIUS, true);
            }
            let _ = ui.set_position(LogicalPosition::new(0.0, 0.0));
            let _ = ui.set_size(LogicalSize::new(ew, eh));
            let _ = ui.show();

            if flags.is_module_running(flags.active_module) {
                if let Some(wv) = app.get_webview(flags.active_module) {
                    show_webview(&wv, (ew, eh));
                    let _ = wv.set_focus();
                } else {
                    let _ = ui.set_focus();
                }
            } else {
                let _ = ui.set_focus();
            }
            if !flags.dialog_open {
                register_escape_shortcut(app);
            }
        } else {
            let _ = ui.show();
        }
    }
}

fn animate_window_to<R: Runtime>(
    app: &AppHandle<R>,
    window: Window<R>,
    ui: Webview<R>,
    target: (f64, f64),
    start_y: f64,
    target_y: f64,
    radius: f64,
    target_expanded: bool,
    resize_active_webview: bool,
) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let current = window.inner_size()?.to_logical::<f64>(scale);
    let (origin_x, screen_w) = monitor_metrics(&window)?;
    let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let app_for_thread = app.clone();

    thread::spawn(move || {
        for step in 1..=ANIM_STEPS {
            let t = ease_out(step as f64 / ANIM_STEPS as f64);
            let width = lerp(current.width, target.0, t);
            let height = lerp(current.height, target.1, t);
            let y = lerp(start_y, target_y, t);
            let _ = window.set_size(LogicalSize::new(width, height));
            let _ = window.set_position(LogicalPosition::new(
                origin_x + centered_x(screen_w, width),
                y,
            ));
            let _ = ui.set_size(LogicalSize::new(width, height));
            if resize_active_webview {
                let flags = current_flags(&app_for_thread);
                if flags.expanded && flags.is_module_running(flags.active_module) {
                    if let Some(wv) = app_for_thread.get_webview(flags.active_module) {
                        let _ = wv.set_size(LogicalSize::new(width, (height - BAR_HEIGHT).max(0.0)));
                    }
                }
            }
            round_window_corners(
                hwnd,
                scale,
                width,
                height,
                radius,
                target_expanded || step < ANIM_STEPS,
            );
            thread::sleep(ANIM_FRAME);
        }
        let flags = current_flags(&app_for_thread);
        if flags.expanded {
            if flags.is_module_running(flags.active_module) {
                hide_other_module_webviews(&app_for_thread, flags.active_module);
                if let Some(wv) = app_for_thread.get_webview(flags.active_module) {
                    show_webview(&wv, active_expanded_size(flags));
                    let _ = wv.set_focus();
                } else if let Some(ui_wv) = app_for_thread.get_webview(UI_LABEL) {
                    let _ = ui_wv.set_focus();
                }
            } else if let Some(ui_wv) = app_for_thread.get_webview(UI_LABEL) {
                let _ = ui_wv.set_focus();
            }
        } else if flags.fullscreen_hidden || flags.user_hidden {
            let _ = sync_visibility(&app_for_thread);
        }
    });
    Ok(())
}

/// Rozwija lub zwija wyspę z animacją rozmiaru.
/// WebView działających aplikacji są jedynie ukrywane przy zwinięciu (bez wyłączania/resetowania).
pub fn set_expanded<R: Runtime>(app: &AppHandle<R>, expanded: bool) -> tauri::Result<()> {
    if !update_flags(app, |f| f.expanded = expanded) {
        return Ok(());
    }
    emit_state(app);

    let (Some(window), Some(ui)) = (
        app.get_window(ISLAND_LABEL),
        app.get_webview(UI_LABEL),
    ) else {
        return Ok(());
    };

    if expanded {
        // Przywołanie wyspy (także z pełnego ekranu): pokaż okno, ustaw na wierzchu, daj fokus i włącz Escape.
        let _ = sync_visibility(app);
        let _ = window.set_focus();
        let flags = current_flags(app);
        if !flags.is_module_running(flags.active_module)
            || flags.active_module == modules::antigravity::LABEL
        {
            let _ = ui.set_focus();
        }
        register_escape_shortcut(app);
    } else {
        // Przy zwinięciu: odrejestruj Escape, wyczyść stan dialogu/fullscreen, ukryj WebView i przytnij Working Set
        unregister_escape_shortcut(app);
        update_flags(app, |f| {
            f.dialog_open = false;
            f.webview_fullscreen = false;
        });
        let _ = ui.show();
        hide_all_module_webviews(app);
        suspend::trim_working_sets_delayed(400);
    }

    let flags = current_flags(app);
    let target = if expanded {
        active_expanded_size(flags)
    } else {
        COLLAPSED_SIZE
    };
    let start_y = if expanded { TOP_MARGIN } else { EXPANDED_TOP_MARGIN };
    let target_y = if expanded { EXPANDED_TOP_MARGIN } else { TOP_MARGIN };
    let radius = if expanded {
        EXPANDED_RADIUS
    } else {
        COLLAPSED_RADIUS
    };
    animate_window_to(app, window, ui, target, start_y, target_y, radius, expanded, false)
}

/// Przełącza między normalnym a powiększonym rozmiarem okna wyspy.
pub fn set_large<R: Runtime>(app: &AppHandle<R>, large: bool) -> tauri::Result<()> {
    if !update_flags(app, |f| f.large = large) {
        return Ok(());
    }
    emit_state(app);

    let flags = current_flags(app);
    if !flags.expanded {
        return Ok(());
    }

    let (Some(window), Some(ui)) = (
        app.get_window(ISLAND_LABEL),
        app.get_webview(UI_LABEL),
    ) else {
        return Ok(());
    };

    animate_window_to(
        app,
        window,
        ui,
        active_expanded_size(flags),
        EXPANDED_TOP_MARGIN,
        EXPANDED_TOP_MARGIN,
        EXPANDED_RADIUS,
        true,
        true,
    )
}

/// Zamyka wskazaną aplikację (moduł): ukrywa jej WebView, nawiguje na about:blank i usypia (lub resetuje sesję CLI).
pub fn close_module<R: Runtime>(app: &AppHandle<R>, module_id: &str) -> tauri::Result<()> {
    let Some(label) = modules::static_label(module_id) else {
        return Ok(());
    };
    if update_flags(app, |f| {
        f.set_module_running(label, false);
    }) {
        let prev = current_flags(app);
        if prev.webview_fullscreen && prev.active_module == label {
            set_webview_fullscreen(app, label, false);
        }
        if prev.active_module == label && is_dialog_open(app) {
            set_dialog_open(app, false);
        }
        if label == modules::antigravity::LABEL {
            app.state::<AntigravitySession>().reset();
        } else {
            if let Some(wv) = app.get_webview(label) {
                let _ = wv.hide();
                if let Ok(blank) = url::Url::parse("about:blank") {
                    let _ = wv.navigate(blank);
                }
            }
            app.state::<SuspendState>().schedule_webview(app, label);
        }
        if current_flags(app).expanded {
            if let Some(ui) = app.get_webview(UI_LABEL) {
                let _ = ui.set_focus();
            }
        }
        suspend::trim_working_sets_delayed(400);
        emit_state(app);
    }
    Ok(())
}

/// Włącza wskazaną aplikację (moduł): wznawia jej WebView, nawiguje na startowy URL i pokazuje widok.
pub fn start_module<R: Runtime>(app: &AppHandle<R>, module_id: &str) -> tauri::Result<()> {
    let Some(label) = modules::static_label(module_id) else {
        return Ok(());
    };
    let mut was_running = false;
    let changed = update_flags(app, |f| {
        was_running = f.is_module_running(label);
        f.active_module = label;
        f.set_module_running(label, true);
    });

    hide_other_module_webviews(app, label);

    if let Some(wv) = app.get_webview(label) {
        if !was_running {
            app.state::<SuspendState>().resume_webview(app, label);
            if let Some(url_str) = modules::start_url(label) {
                if let Ok(url) = url::Url::parse(url_str) {
                    let _ = wv.navigate(url);
                }
            }
        }
        let flags = current_flags(app);
        if flags.expanded {
            show_webview(&wv, active_expanded_size(flags));
            let _ = wv.set_focus();
        } else {
            let _ = wv.hide();
            set_expanded(app, true)?;
        }
    } else if !current_flags(app).expanded {
        set_expanded(app, true)?;
    } else if let Some(ui) = app.get_webview(UI_LABEL) {
        let _ = ui.set_focus();
    }

    suspend::trim_working_sets_delayed(1500);

    if changed {
        emit_state(app);
    }
    Ok(())
}

/// Przełącza aktywny widok na wskazany moduł (bez przeładowywania, jeśli już działa).
pub fn activate_module<R: Runtime>(app: &AppHandle<R>, module_id: &str) -> tauri::Result<()> {
    let Some(label) = modules::static_label(module_id) else {
        return Ok(());
    };
    let prev = current_flags(app);
    if prev.webview_fullscreen && prev.active_module != label {
        set_webview_fullscreen(app, prev.active_module, false);
    }
    let changed = update_flags(app, |f| {
        f.active_module = label;
    });

    hide_other_module_webviews(app, label);

    let flags = current_flags(app);
    if flags.expanded {
        if flags.is_module_running(label) {
            if let Some(wv) = app.get_webview(label) {
                show_webview(&wv, active_expanded_size(flags));
                let _ = wv.set_focus();
            } else if let Some(ui) = app.get_webview(UI_LABEL) {
                let _ = ui.set_focus();
            }
        } else if let Some(ui) = app.get_webview(UI_LABEL) {
            let _ = ui.set_focus();
        }
    } else {
        set_expanded(app, true)?;
    }

    suspend::trim_working_sets_delayed(400);

    if changed {
        emit_state(app);
    }
    Ok(())
}

/// Przełącza pomiędzy modułami na pasku (Tab / Shift+Tab).
pub fn cycle_module<R: Runtime>(app: &AppHandle<R>, delta: i32) -> tauri::Result<()> {
    let flags = current_flags(app);
    if !flags.expanded || flags.dialog_open || flags.webview_fullscreen {
        return Ok(());
    }
    let list = app.state::<modules::ModuleRegistry>().list();
    if list.is_empty() {
        return Ok(());
    }
    let current_idx = list
        .iter()
        .position(|m| m.id == flags.active_module)
        .unwrap_or(0) as isize;
    let len = list.len() as isize;
    let next_idx = ((current_idx + delta as isize) % len + len) % len;
    activate_module(app, list[next_idx as usize].id)
}

/// Czyści ciasteczka, cache i pamięć lokalną wspólnego profilu oraz aktywnych aplikacji.
pub fn logout_all<R: Runtime>(app: &AppHandle<R>) {
    if let Some(ui_wv) = app.get_webview(UI_LABEL) {
        if let Err(e) = ui_wv.clear_all_browsing_data() {
            eprintln!("nie udało się wyczyścić danych profilu ({UI_LABEL}): {e}");
        }
    }
    let flags = current_flags(app);
    for &label in modules::WEBVIEW_LABELS {
        if let Some(webview) = app.get_webview(label) {
            if let Err(e) = webview.clear_all_browsing_data() {
                eprintln!("nie udało się wyczyścić danych przeglądarki ({label}): {e}");
            }
            if flags.is_module_running(label) {
                if let Some(url_str) = modules::start_url(label) {
                    if let Ok(url) = url::Url::parse(url_str) {
                        let _ = webview.navigate(url);
                    }
                }
            }
        }
    }
}

pub fn toggle_expanded<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if current_flags(app).user_hidden {
        set_user_hidden(app, false)?;
    }
    let expanded = current_flags(app).expanded;
    set_expanded(app, !expanded)
}

/// Pokazuje lub ukrywa okno wyspy zgodnie z flagami (ręczne ukrycie ma pierwszeństwo nad wykryciem gry).
fn sync_visibility<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let flags = current_flags(app);
    let Some(window) = app.get_window(ISLAND_LABEL) else {
        return Ok(());
    };
    if flags.user_hidden || (flags.fullscreen_hidden && !flags.expanded) {
        window.set_always_on_top(false)?;
        window.hide()?;
    } else {
        window.set_always_on_top(true)?;
        window.show()?;
    }
    Ok(())
}

pub fn set_user_hidden<R: Runtime>(app: &AppHandle<R>, hidden: bool) -> tauri::Result<()> {
    if update_flags(app, |f| f.user_hidden = hidden) {
        sync_visibility(app)?;
        emit_state(app);
    }
    Ok(())
}

/// Przełącza ręczne ukrycie; zwraca nowy stan.
pub fn toggle_user_hidden<R: Runtime>(app: &AppHandle<R>) -> bool {
    let hidden = !current_flags(app).user_hidden;
    let _ = set_user_hidden(app, hidden);
    hidden
}

pub fn set_fullscreen_hidden<R: Runtime>(app: &AppHandle<R>, hidden: bool) {
    if update_flags(app, |f| f.fullscreen_hidden = hidden) {
        let _ = sync_visibility(app);
        emit_state(app);
    }
}
