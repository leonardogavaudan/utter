use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Wry};

use crate::events::Phase;
use crate::windows;

const TRAY_ID: &str = "utter";
const SETTINGS_ITEM: &str = "settings";
const QUIT_ITEM: &str = "quit";

const IDLE_ICON: &[u8] = include_bytes!("../icons/tray/idle/44x44.png");
const RECORDING_ICON: &[u8] = include_bytes!("../icons/tray/recording/44x44.png");
const PROCESSING_ICON: &[u8] = include_bytes!("../icons/tray/processing/44x44.png");

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let menu = Menu::<Wry>::with_items(
        app,
        &[
            &MenuItem::with_id(app, SETTINGS_ITEM, "Settings…", true, Some("CmdOrCtrl+,"))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, QUIT_ITEM, "Quit Utter", true, Some("CmdOrCtrl+Q"))?,
        ],
    )?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(IDLE_ICON)?)
        .icon_as_template(true)
        .tooltip(tooltip(Phase::Idle))
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .build(app)?;
    Ok(())
}

pub fn show_phase(app: &AppHandle, phase: Phase) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let icon = match phase {
        Phase::Idle => IDLE_ICON,
        Phase::Recording => RECORDING_ICON,
        Phase::Processing => PROCESSING_ICON,
    };
    let updated = Image::from_bytes(icon)
        .and_then(|image| tray.set_icon(Some(image)))
        .and_then(|()| tray.set_icon_as_template(phase != Phase::Recording))
        .and_then(|()| tray.set_tooltip(Some(tooltip(phase))));
    if let Err(error) = updated {
        log::warn!("could not update the tray icon: {error}");
    }
}

fn tooltip(phase: Phase) -> &'static str {
    match phase {
        Phase::Idle => "Utter",
        Phase::Recording => "Utter: listening",
        Phase::Processing => "Utter: transcribing",
    }
}

fn handle_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        SETTINGS_ITEM => windows::show_settings(app),
        QUIT_ITEM => app.exit(0),
        _ => {}
    }
}
