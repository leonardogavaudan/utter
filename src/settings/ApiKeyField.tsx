import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import { API_KEYS_CHANGED, api, errorMessage } from "../api";
import type { ProviderPreset } from "../bindings/ProviderPreset";
import type { Role } from "../bindings/Role";
import { Field } from "./Layout";

export function ApiKeyField({ preset, providerRole }: { preset: ProviderPreset; providerRole: Role }) {
  const [saved, setSaved] = useState<boolean | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    api
      .hasApiKey(preset.id, providerRole)
      .then(setSaved)
      .catch((reason: unknown) => setError(errorMessage(reason)));
  }, [preset.id, providerRole]);

  useEffect(() => {
    setDraft("");
    setError(null);
    refresh();
    window.addEventListener(API_KEYS_CHANGED, refresh);
    return () => window.removeEventListener(API_KEYS_CHANGED, refresh);
  }, [refresh]);

  const run = async (action: () => Promise<void>) => {
    try {
      await action();
      setError(null);
      setDraft("");
      window.dispatchEvent(new Event(API_KEYS_CHANGED));
    } catch (reason) {
      setError(errorMessage(reason));
    }
  };

  const save = () => run(() => api.setApiKey(preset.id, providerRole, draft));
  const remove = () => run(() => api.deleteApiKey(preset.id, providerRole));

  const hint = error ? (
    <span className="is-error">{error}</span>
  ) : (
    <>
      {saved ? "Stored in your Keychain. " : "Kept in your Keychain, never in the settings file. "}
      {preset.apiKeyUrl && (
        <button type="button" className="link" onClick={() => void openUrl(preset.apiKeyUrl ?? "")}>
          Get a {preset.label} key
        </button>
      )}
    </>
  );

  return (
    <Field label={preset.requiresApiKey ? "API key" : "API key (optional)"} hint={hint}>
      {(control) => (
        <span className="inline">
          <input
            {...control}
            type="password"
            value={draft}
            placeholder={saved ? "••••••••••••  saved" : "Paste your API key"}
            autoComplete="off"
            spellCheck={false}
            onChange={(event) => setDraft(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Enter" && draft.trim()) {
                event.preventDefault();
                void save();
              }
            }}
          />
          <button type="button" disabled={!draft.trim()} onClick={() => void save()}>
            Save
          </button>
          {saved && (
            <button type="button" className="quiet" onClick={() => void remove()}>
              Remove
            </button>
          )}
        </span>
      )}
    </Field>
  );
}
