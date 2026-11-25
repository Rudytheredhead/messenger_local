use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Runtime,
};

use crate::core::window_manager;

pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Pokaż wyspę", true, None::<&str>)?;
    let hide = CheckMenuItem::with_id(
        app,
        "toggle_hidden",
        "Nie przeszkadzaj (ukryj wyspę)",
        true,
        false,
        None::<&str>,
    )?;
    let logout = MenuItem::with_id(app, "logout", "Wyloguj / Wyczyść sesję", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Zakończ", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &hide, &logout, &quit])?;

    let hide_item = hide.clone();
    let mut tray = TrayIconBuilder::with_id("main-tray")
        .tooltip("TopIsland")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => {
                let _ = window_manager::set_user_hidden(app, false);
            }
            "toggle_hidden" => {
                let hidden = window_manager::toggle_user_hidden(app);
                let _ = hide_item.set_checked(hidden);
            }
            "logout" => window_manager::logout_all(app),
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
                let _ = window_manager::set_user_hidden(tray.app_handle(), false);
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}
