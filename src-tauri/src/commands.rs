use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use ts_rs::TS;
use utter_core::microphone::Microphone;
use utter_core::pipeline::Dictation;
use utter_core::providers::{self, PRESETS, ProviderId, ProviderPreset, Role};
use utter_core::settings::Settings;
use utter_os::permissions::{self, MicrophoneAccess, Permission};

use crate::error::{AppError, AppResult};
use crate::events::Phase;
use crate::secrets::ApiKeySource;
use crate::shortcut;
use crate::state::AppState;

#[derive(Clone, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Permissions {
    pub accessibility: bool,
    pub microphone: MicrophoneStatus,
}

#[derive(Clone, Copy, Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MicrophoneStatus {
    NotDetermined,
    Denied,
    Granted,
}

impl From<MicrophoneAccess> for MicrophoneStatus {
    fn from(access: MicrophoneAccess) -> MicrophoneStatus {
        match access {
            MicrophoneAccess::NotDetermined => MicrophoneStatus::NotDetermined,
            MicrophoneAccess::Denied => MicrophoneStatus::Denied,
            MicrophoneAccess::Granted => MicrophoneStatus::Granted,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PermissionKind {
    Accessibility,
    Microphone,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings().clone()
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> AppResult<Settings> {
    let settings = settings.normalized();
    settings.validate()?;
    let previous_shortcut = state.settings().shortcut.clone();
    let shortcut_changed = settings.shortcut != previous_shortcut;
    if shortcut_changed {
        shortcut::register(&app, Some(&previous_shortcut), &settings.shortcut)?;
    }
    if let Err(error) = settings.save(&state.settings_path) {
        if shortcut_changed {
            let _ = shortcut::register(&app, Some(&settings.shortcut), &previous_shortcut);
        }
        return Err(error.into());
    }
    if shortcut_changed {
        *state.shortcut_error() = None;
    }
    *state.settings_mut() = settings.clone();
    Ok(settings)
}

#[tauri::command]
pub fn list_providers() -> &'static [ProviderPreset] {
    PRESETS
}

#[tauri::command]
pub async fn list_models(
    state: State<'_, AppState>,
    provider: ProviderId,
    role: Role,
    base_url: String,
) -> AppResult<Vec<String>> {
    let preset = provider.preset();
    let api_key = state.secrets.get(provider, role);
    if api_key.is_none() && preset.requires_api_key {
        return Err(utter_core::Error::MissingApiKey(preset.label.to_owned()).into());
    }
    Ok(providers::list_models(&state.http, preset.api, &base_url, api_key.as_deref()).await?)
}

#[tauri::command]
pub fn get_api_key_source(
    state: State<'_, AppState>,
    provider: ProviderId,
    role: Role,
) -> ApiKeySource {
    state.secrets.source(provider, role)
}

#[tauri::command]
pub fn set_api_key(
    state: State<'_, AppState>,
    provider: ProviderId,
    role: Role,
    api_key: String,
) -> AppResult<()> {
    let api_key = api_key.trim();
    if api_key.is_empty() {
        return Err(AppError::Internal("the API key is empty".to_owned()));
    }
    state.secrets.set(provider, role, api_key)
}

#[tauri::command]
pub fn delete_api_key(
    state: State<'_, AppState>,
    provider: ProviderId,
    role: Role,
) -> AppResult<()> {
    state.secrets.delete(provider, role)
}

#[tauri::command]
pub async fn list_microphones() -> AppResult<Vec<Microphone>> {
    tauri::async_runtime::spawn_blocking(utter_os::microphones::list_microphones)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
pub fn get_permissions() -> Permissions {
    Permissions {
        accessibility: permissions::accessibility_granted(),
        microphone: permissions::microphone_access().into(),
    }
}

#[tauri::command]
pub async fn request_microphone() -> AppResult<MicrophoneStatus> {
    tauri::async_runtime::spawn_blocking(permissions::request_microphone)
        .await
        .map(MicrophoneStatus::from)
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
pub fn request_accessibility() -> bool {
    permissions::request_accessibility()
}

#[tauri::command]
pub fn open_permission_settings(app: AppHandle, permission: PermissionKind) -> AppResult<()> {
    let permission = match permission {
        PermissionKind::Accessibility => Permission::Accessibility,
        PermissionKind::Microphone => Permission::Microphone,
    };
    let Some(url) = permissions::settings_url(permission) else {
        return Ok(());
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|error| AppError::Internal(error.to_string()))
}

#[tauri::command]
pub fn get_shortcut_error(state: State<'_, AppState>) -> Option<String> {
    state.shortcut_error().clone()
}

#[tauri::command]
pub fn get_pending_update(state: State<'_, AppState>) -> Option<String> {
    state.pending_update().clone()
}

#[tauri::command]
pub fn restart_to_update(app: AppHandle) {
    app.restart();
}

#[tauri::command]
pub fn get_phase(state: State<'_, AppState>) -> Phase {
    state.session().phase()
}

#[tauri::command]
pub fn get_last_dictation(state: State<'_, AppState>) -> Option<Dictation> {
    state.last_dictation().clone()
}

#[tauri::command]
pub fn copy_last_dictation(state: State<'_, AppState>) -> AppResult<()> {
    let Some(text) = state
        .last_dictation()
        .as_ref()
        .map(|dictation| dictation.text.trim().to_owned())
    else {
        return Ok(());
    };
    utter_os::insert::copy_to_clipboard(&text)?;
    Ok(())
}
