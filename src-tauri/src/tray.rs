//! System tray icon, menu, and close-to-tray behaviour.
//!
//! Entirely opt-in: with `general.trayEnabled` off there is no tray item and
//! closing the window quits, which is what a Linux user who never asked for
//! a tray expects. A desktop without a StatusNotifier host (some tiling WMs,
//! bare X sessions) has nowhere to put the icon, so failing to create it is
//! logged and otherwise ignored rather than taken as a startup error — the
//! app must still run.

use crate::db::repo::settings as settings_repo;
use crate::settings_defaults as sk;
use crate::state::AppState;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, Runtime,
};

/// Is closing the window meant to hide it rather than quit?
pub fn close_to_tray(state: &AppState) -> bool {
    settings_repo::get_or(&state.db.conn(), sk::TRAY_ENABLED, false)
}

fn show_main<R: Runtime>(app: &AppHandle<R>) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Build the tray icon, if the user asked for one.
pub fn install<R: Runtime>(app: &AppHandle<R>, state: &Arc<AppState>) {
    if !close_to_tray(state) {
        return;
    }

    let open = MenuItem::with_id(app, "open", "Open OpenClaude", true, None::<&str>);
    let new_chat = MenuItem::with_id(app, "new", "New conversation", true, None::<&str>);
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>);
    let (open, new_chat, quit) = match (open, new_chat, quit) {
        (Ok(a), Ok(b), Ok(c)) => (a, b, c),
        _ => {
            tracing::warn!("could not build the tray menu; continuing without a tray");
            return;
        }
    };

    let menu = match Menu::with_items(app, &[&open, &new_chat, &quit]) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!(error = %e, "could not build the tray menu");
            return;
        }
    };

    let built = TrayIconBuilder::with_id("main")
        .tooltip("OpenClaude Desktop")
        .icon(app.default_window_icon().cloned().unwrap_or_else(|| {
            // Should not happen — the icon is compiled in — but a missing
            // icon must not take the tray down with it.
            tauri::image::Image::new_owned(vec![0, 0, 0, 0], 1, 1)
        }))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "new" => {
                show_main(app);
                // The window owns conversation creation; the tray only asks.
                let _ = app.emit_to("main", "tray:new-conversation", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left click raises the window, which is the convention users
            // expect; the menu stays on right click.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app);

    if let Err(e) = built {
        tracing::warn!(error = %e, "no system tray available; the app runs without one");
    }
}
