import { openUrl } from "@tauri-apps/plugin-opener";
import { useCallback, useEffect, useState } from "react";
import { API_KEYS_CHANGED, api, errorMessage } from "../api";
import type { ApiKeySource } from "../bindings/ApiKeySource";
import type { ProviderPreset } from "../bindings/ProviderPreset";
import type { Role } from "../bindings/Role";
import { Field } from "./Layout";

export function ApiKeyField({
  preset,
  providerRole,
  apiKeyVariable,
}: {
  preset: ProviderPreset;
  providerRole: Role;
  apiKeyVariable: string | null;
}) {
  const [source, setSource] = useState<ApiKeySource | null>(null);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(() => {
    api
      .getApiKeySource(preset.id, providerRole, apiKeyVariable)
      .then(setSource)
      .catch((reason: unknown) => setError(errorMessage(reason)));
  }, [preset.id, providerRole, apiKeyVariable]);

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
  const label = preset.requiresApiKey ? "API key" : "API key (optional)";
  const keyLink = preset.apiKeyUrl && (
    <button type="button" className="link" onClick={() => void openUrl(preset.apiKeyUrl ?? "")}>
      Get a {preset.label} key
    </button>
  );

  if (source?.kind === "environment") {
    return (
      <Field
        label={label}
        hint={`Utter reads this key from the ${source.variable} environment variable, so it never touches your Keychain.`}
      >
        {(control) => <input {...control} type="text" readOnly value={`From ${source.variable}`} />}
      </Field>
    );
  }

  const saved = source?.kind === "keychain";
  const hint = error ? (
    <span className="is-error">{error}</span>
  ) : (
    <>
      {saved ? "Stored in your Keychain. " : "Kept in your Keychain, never in the settings file. "}
      {keyLink}
    </>
  );

  return (
    <Field label={label} hint={hint}>
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
