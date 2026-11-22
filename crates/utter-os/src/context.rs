use utter_core::context::{ContextMode, TextContext};

#[cfg(target_os = "macos")]
pub fn capture(mode: ContextMode) -> TextContext {
    crate::macos::context::capture(mode)
}

#[cfg(not(target_os = "macos"))]
pub fn capture(_mode: ContextMode) -> TextContext {
    TextContext::default()
}

#[cfg(target_os = "macos")]
pub fn focused_app_pid() -> Option<i32> {
    crate::macos::context::focused_app_pid()
}

#[cfg(not(target_os = "macos"))]
pub fn focused_app_pid() -> Option<i32> {
    None
}

#[cfg(target_os = "macos")]
pub fn is_text_field_focused() -> bool {
    crate::macos::context::is_text_field_focused()
}

#[cfg(not(target_os = "macos"))]
pub fn is_text_field_focused() -> bool {
    true
}
