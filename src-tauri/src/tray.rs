use tauri::image::Image;
use tauri::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Wry};

use crate::events::Phase;
use crate::windows;

const TRAY_ID: &str = "utter";
const SETTINGS_ITEM: &str = "settings";
const RESTART_ITEM: &str = "restart";
const QUIT_ITEM: &str = "quit";

const IDLE_ICON: &[u8] = include_bytes!("../icons/tray/idle/44x44.png");
const RECORDING_ICON: &[u8] = include_bytes!("../icons/tray/recording/44x44.png");
const PROCESSING_ICON: &[u8] = include_bytes!("../icons/tray/processing/44x44.png");

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(IDLE_ICON)?)
        .icon_as_template(true)
        .tooltip(tooltip(Phase::Idle))
        .menu(&build_menu(app, None)?)
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

pub fn show_update_ready(app: &AppHandle, version: &str) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };
    let updated = build_menu(app, Some(version)).and_then(|menu| tray.set_menu(Some(menu)));
    if let Err(error) = updated {
        log::warn!("could not add the restart item to the tray menu: {error}");
    }
}

fn build_menu(app: &AppHandle, pending_update: Option<&str>) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::with_items(
        app,
        &[
            &MenuItem::with_id(app, SETTINGS_ITEM, "Settings…", true, Some("CmdOrCtrl+,"))?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, QUIT_ITEM, "Quit Utter", true, Some("CmdOrCtrl+Q"))?,
        ],
    )?;
    if let Some(version) = pending_update {
        let restart = MenuItem::with_id(
            app,
            RESTART_ITEM,
            format!("Restart to Update to {version}"),
            true,
            None::<&str>,
        )?;
        menu.prepend_items(&[&restart, &PredefinedMenuItem::separator(app)?])?;
    }
    Ok(menu)
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
        RESTART_ITEM => app.restart(),
        QUIT_ITEM => app.exit(0),
        _ => {}
    }
}
