use std::time::Duration;

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::UpdaterExt;

use crate::events;
use crate::state::AppState;
use crate::tray;

const FIRST_CHECK_DELAY: Duration = Duration::from_secs(30);
const CHECK_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

pub fn check_periodically(app: &AppHandle) {
    if cfg!(debug_assertions) {
        return;
    }
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(FIRST_CHECK_DELAY).await;
        loop {
            if let Err(error) = install_available_update(&app).await {
                log::warn!("update check failed: {error}");
            }
            tokio::time::sleep(CHECK_INTERVAL).await;
        }
    });
}

async fn install_available_update(app: &AppHandle) -> tauri_plugin_updater::Result<()> {
    if app.state::<AppState>().pending_update().is_some() {
        return Ok(());
    }
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };
    log::info!("installing Utter {}", update.version);
    update.download_and_install(|_, _| {}, || {}).await?;
    *app.state::<AppState>().pending_update() = Some(update.version.clone());
    tray::show_update_ready(app, &update.version);
    events::update_ready(app, &update.version);
    Ok(())
}
