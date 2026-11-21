use std::ptr::NonNull;

use block2::RcBlock;
use objc2_app_kit::NSApplicationDidBecomeActiveNotification;
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue};
use tauri::AppHandle;

use crate::windows;

pub fn show_settings_when_activated(app: &AppHandle) {
    let app = app.clone();
    let handler = RcBlock::new(move |_notification: NonNull<NSNotification>| {
        windows::show_settings_if_hidden(&app);
    });
    let observer = unsafe {
        NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
            Some(NSApplicationDidBecomeActiveNotification),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &handler,
        )
    };
    std::mem::forget(observer);
}
