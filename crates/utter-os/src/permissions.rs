#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Accessibility,
    Microphone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MicrophoneAccess {
    NotDetermined,
    Denied,
    Granted,
}

#[cfg(target_os = "macos")]
pub fn accessibility_granted() -> bool {
    crate::macos::permissions::accessibility_granted(false)
}

#[cfg(target_os = "macos")]
pub fn request_accessibility() -> bool {
    crate::macos::permissions::accessibility_granted(true)
}

#[cfg(target_os = "macos")]
pub fn microphone_access() -> MicrophoneAccess {
    crate::macos::permissions::microphone_access()
}

#[cfg(target_os = "macos")]
pub fn request_microphone() -> MicrophoneAccess {
    crate::macos::permissions::request_microphone()
}

#[cfg(target_os = "macos")]
pub fn settings_url(permission: Permission) -> Option<&'static str> {
    Some(match permission {
        Permission::Accessibility => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
        }
        Permission::Microphone => {
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        }
    })
}

#[cfg(not(target_os = "macos"))]
pub fn accessibility_granted() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn request_accessibility() -> bool {
    true
}

#[cfg(not(target_os = "macos"))]
pub fn microphone_access() -> MicrophoneAccess {
    MicrophoneAccess::Granted
}

#[cfg(not(target_os = "macos"))]
pub fn request_microphone() -> MicrophoneAccess {
    MicrophoneAccess::Granted
}

#[cfg(not(target_os = "macos"))]
pub fn settings_url(_permission: Permission) -> Option<&'static str> {
    None
}
