use tauri::{AppHandle, Manager, Window, WindowEvent};

const SETTINGS_LABEL: &str = "settings";

pub fn show_settings(app: &AppHandle) {
    let Some(window) = app.get_webview_window(SETTINGS_LABEL) else {
        log::error!("the settings window is missing");
        return;
    };
    if !window.is_visible().unwrap_or(false)
        && let Err(error) = window.reload()
    {
        log::warn!("could not reload the settings window: {error}");
    }
    let shown = window
        .show()
        .and_then(|()| window.unminimize())
        .and_then(|()| window.set_focus());
    if let Err(error) = shown {
        log::warn!("could not show the settings window: {error}");
    }
}

pub fn show_settings_if_hidden(app: &AppHandle) {
    let hidden = app
        .get_webview_window(SETTINGS_LABEL)
        .is_some_and(|window| !window.is_visible().unwrap_or(false));
    if hidden {
        show_settings(app);
    }
}

pub fn handle_event(window: &Window, event: &WindowEvent) {
    if window.label() == SETTINGS_LABEL
        && let WindowEvent::CloseRequested { api, .. } = event
    {
        api.prevent_close();
        let _ = window.hide();
    }
}
