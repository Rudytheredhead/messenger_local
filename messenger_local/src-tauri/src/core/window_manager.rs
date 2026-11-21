use std::sync::Arc;
use std::thread;
use std::time::Duration;

use island_core::{
    centered_x, ease_out, lerp, BAR_HEIGHT, COLLAPSED_SIZE, EXPANDED_SIZE, TOP_MARGIN,
};
use serde::Serialize;
use tauri::{
    AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Runtime, Window,
    WindowBuilder,
};
use tauri::webview::WebviewBuilder;
use tauri::WebviewUrl;

use crate::core::state::{IslandFlags, IslandStore};
use crate::core::suspend;
use crate::modules::messenger::{self, Messenger};
use crate::modules::ModuleRegistry;

pub const ISLAND_LABEL: &str = "island";
pub const UI_LABEL: &str = "island-ui";
const UI_PAGE: &str = "index.html";
/// Przezroczyste tło WebView (domyślnie biały prostokąt wokół pigułki).
const TRANSPARENT: tauri::window::Color = tauri::window::Color(0, 0, 0, 0);
pub const STATE_EVENT: &str = "island://state";

const ANIM_STEPS: u32 = 10;
/// Promienie dolnych rogów — zgodne z CSS (`styles.css`).
const COLLAPSED_RADIUS: f64 = 10.0;
const EXPANDED_RADIUS: f64 = 18.0;
const ANIM_FRAME: Duration = Duration::from_millis(16);

#[derive(Serialize, Clone, Debug)]
pub struct StatusDto {
    pub id: String,
    pub icon: String,
    pub badge: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct IslandState {
    pub expanded: bool,
    pub user_hidden: bool,
    pub fullscreen_hidden: bool,
    pub status: Option<StatusDto>,
}

/// Zaokrągla dolne rogi okna regionem Win32. Region obcina też child WebView-y (Messenger),
/// więc CSS nie wystarcza. Górna krawędź jest rozciągnięta o promień poza okno, dzięki czemu
/// górne rogi są poza ekranem, a tylko dolne są zaokrąglone.
#[cfg(windows)]
fn round_bottom_corners(hwnd: isize, scale: f64, width: f64, height: f64, radius: f64) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
    if hwnd == 0 {
        return;
    }
    let r = (radius * scale).round() as i32;
    let w = (width * scale).round() as i32;
    let h = (height * scale).round() as i32;
    unsafe {
        let region = CreateRoundRectRgn(0, -r, w, h, 2 * r, 2 * r);
        // System przejmuje region po udanym SetWindowRgn — nie zwalniamy go ręcznie.
        SetWindowRgn(HWND(hwnd as *mut std::ffi::c_void), Some(region), true);
    }
}

#[cfg(not(windows))]
fn round_bottom_corners(_hwnd: isize, _scale: f64, _width: f64, _height: f64, _radius: f64) {}

pub fn snapshot<R: Runtime>(app: &AppHandle<R>) -> IslandState {
    let flags = current_flags(app);
    let status = app
        .state::<ModuleRegistry>()
        .status()
        .map(|s| StatusDto { id: s.id.to_string(), icon: s.icon.to_string(), badge: s.badge });
    IslandState {
        expanded: flags.expanded,
        user_hidden: flags.user_hidden,
        fullscreen_hidden: flags.fullscreen_hidden,
        status,
    }
}

pub fn emit_state<R: Runtime>(app: &AppHandle<R>) {
    let _ = app.emit(STATE_EVENT, snapshot(app));
}

pub fn is_expanded<R: Runtime>(app: &AppHandle<R>) -> bool {
    current_flags(app).expanded
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

/// Tworzy okno wyspy (bez ramki, przezroczyste, zawsze na wierzchu), jej WebView z interfejsem
/// i WebView Messengera. Messenger ładuje się od razu, żeby licznik nieprzeczytanych działał także
/// przy zwiniętej wyspie; podczas zwinięcia jest tylko ukryty.
pub fn create<R: Runtime>(app: &AppHandle<R>, messenger: Arc<Messenger>) -> tauri::Result<()> {
    let (w, h) = COLLAPSED_SIZE;
    let window = WindowBuilder::new(app, ISLAND_LABEL)
        .title("Messenger Island")
        .inner_size(w, h)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .content_protected(true)
        .build()?;

    let profile = app.path().app_local_data_dir()?.join(messenger::PROFILE_DIR);
    let ui = WebviewBuilder::new(UI_LABEL, WebviewUrl::App(UI_PAGE.into()))
        .transparent(true)
        .background_color(TRANSPARENT)
        .data_directory(profile.clone());
    window.add_child(ui, LogicalPosition::new(0.0, 0.0), LogicalSize::new(w, h))?;

    let (ew, eh) = EXPANDED_SIZE;
    window.add_child(
        messenger::builder(app, messenger)?,
        LogicalPosition::new(0.0, BAR_HEIGHT),
        LogicalSize::new(ew, eh - BAR_HEIGHT),
    )?;
    if let Some(msg) = app.get_webview(messenger::LABEL) {
        msg.hide()?;
    }
    // Wyspa startuje zwinięta — zaplanuj ewentualne uśpienie strony.
    suspend::schedule(app);

    let (origin_x, screen_w) = monitor_metrics(&window)?;
    window.set_position(LogicalPosition::new(origin_x + centered_x(screen_w, w), TOP_MARGIN))?;
    let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    round_bottom_corners(hwnd, window.scale_factor()?, w, h, COLLAPSED_RADIUS);
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

/// Rozwija lub zwija wyspę z animacją rozmiaru. Messenger jest ukrywany przed zwinięciem
/// i pokazywany po rozwinięciu (w poprawionej geometrii pod paskiem zakładek).
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
    let msg = app.get_webview(messenger::LABEL);
    if expanded {
        suspend::resume(app);
    } else {
        if let Some(m) = &msg {
            m.hide()?;
        }
        suspend::schedule(app);
    }

    let scale = window.scale_factor()?;
    let current = window.inner_size()?.to_logical::<f64>(scale);
    let target = if expanded { EXPANDED_SIZE } else { COLLAPSED_SIZE };
    let (origin_x, screen_w) = monitor_metrics(&window)?;
    let hwnd = window.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let radius = if expanded { EXPANDED_RADIUS } else { COLLAPSED_RADIUS };

    thread::spawn(move || {
        for step in 1..=ANIM_STEPS {
            let t = ease_out(step as f64 / ANIM_STEPS as f64);
            let width = lerp(current.width, target.0, t);
            let height = lerp(current.height, target.1, t);
            let _ = window.set_size(LogicalSize::new(width, height));
            let _ = window.set_position(LogicalPosition::new(
                origin_x + centered_x(screen_w, width),
                TOP_MARGIN,
            ));
            let _ = ui.set_size(LogicalSize::new(width, height));
            round_bottom_corners(hwnd, scale, width, height, radius);
            thread::sleep(ANIM_FRAME);
        }
        if let (true, Some(m)) = (expanded, msg) {
            let _ = m.set_position(LogicalPosition::new(0.0, BAR_HEIGHT));
            let _ = m.set_size(LogicalSize::new(EXPANDED_SIZE.0, EXPANDED_SIZE.1 - BAR_HEIGHT));
            let _ = m.show();
        }
    });
    Ok(())
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
    if flags.user_hidden || flags.fullscreen_hidden {
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
