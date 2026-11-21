use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewWindow};

const LABEL: &str = "overlay";
const NOTICE_DURATION: Duration = Duration::from_millis(2_800);
const BOTTOM_MARGIN: f64 = 72.0;

static VISIBILITY_GENERATION: AtomicU64 = AtomicU64::new(0);

pub fn prepare(app: &AppHandle) {
    if let Some(window) = window(app)
        && let Err(error) = window.set_ignore_cursor_events(true)
    {
        log::warn!("the overlay will intercept clicks: {error}");
    }
}

pub fn show(app: &AppHandle) {
    VISIBILITY_GENERATION.fetch_add(1, Ordering::SeqCst);
    let Some(window) = window(app) else {
        return;
    };
    place_at_bottom_center(app, &window);
    if let Err(error) = window.show() {
        log::warn!("could not show the overlay: {error}");
    }
}

pub fn hide(app: &AppHandle) {
    VISIBILITY_GENERATION.fetch_add(1, Ordering::SeqCst);
    if let Some(window) = window(app) {
        let _ = window.hide();
    }
}

pub fn hide_later(app: &AppHandle) {
    let generation = VISIBILITY_GENERATION.load(Ordering::SeqCst);
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(NOTICE_DURATION).await;
        if VISIBILITY_GENERATION.load(Ordering::SeqCst) == generation {
            hide(&app);
        }
    });
}

fn window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(LABEL)
}

fn place_at_bottom_center(app: &AppHandle, window: &WebviewWindow) {
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|cursor| app.monitor_from_point(cursor.x, cursor.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        return;
    };
    let area = monitor.work_area();
    let margin = (BOTTOM_MARGIN * monitor.scale_factor()) as i32;
    let x = area.position.x + (area.size.width as i32 - size.width as i32) / 2;
    let y = area.position.y + area.size.height as i32 - size.height as i32 - margin;
    let _ = window.set_position(PhysicalPosition::new(x, y));
}
