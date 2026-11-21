import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Dictation } from "./bindings/Dictation";
import type { MicrophoneStatus } from "./bindings/MicrophoneStatus";
import type { Notice } from "./bindings/Notice";
import type { PermissionKind } from "./bindings/PermissionKind";
import type { Permissions } from "./bindings/Permissions";
import type { Phase } from "./bindings/Phase";
import type { ProviderId } from "./bindings/ProviderId";
import type { ProviderPreset } from "./bindings/ProviderPreset";
import type { Role } from "./bindings/Role";
import type { Settings } from "./bindings/Settings";

export const API_KEYS_CHANGED = "utter:api-keys-changed";

export const api = {
  getSettings: () => invoke<Settings>("get_settings"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  listProviders: () => invoke<ProviderPreset[]>("list_providers"),
  listModels: (provider: ProviderId, role: Role, baseUrl: string) =>
    invoke<string[]>("list_models", { provider, role, baseUrl }),
  hasApiKey: (provider: ProviderId, role: Role) => invoke<boolean>("has_api_key", { provider, role }),
  setApiKey: (provider: ProviderId, role: Role, apiKey: string) =>
    invoke<void>("set_api_key", { provider, role, apiKey }),
  deleteApiKey: (provider: ProviderId, role: Role) => invoke<void>("delete_api_key", { provider, role }),
  getPermissions: () => invoke<Permissions>("get_permissions"),
  requestAccessibility: () => invoke<boolean>("request_accessibility"),
  requestMicrophone: () => invoke<MicrophoneStatus>("request_microphone"),
  openPermissionSettings: (permission: PermissionKind) => invoke<void>("open_permission_settings", { permission }),
  getShortcutError: () => invoke<string | null>("get_shortcut_error"),
  getPhase: () => invoke<Phase>("get_phase"),
  getLastDictation: () => invoke<Dictation | null>("get_last_dictation"),
};

export const events = {
  onPhase: (handler: (phase: Phase) => void): Promise<UnlistenFn> =>
    listen<Phase>("dictation-phase", (event) => handler(event.payload)),
  onNotice: (handler: (notice: Notice) => void): Promise<UnlistenFn> =>
    listen<Notice>("dictation-notice", (event) => handler(event.payload)),
  onDictation: (handler: (dictation: Dictation) => void): Promise<UnlistenFn> =>
    listen<Dictation>("dictation-finished", (event) => handler(event.payload)),
};

export function errorMessage(error: unknown): string {
  const message = typeof error === "string" ? error : error instanceof Error ? error.message : String(error);
  return message.charAt(0).toUpperCase() + message.slice(1);
}
