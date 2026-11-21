import { useCallback, useEffect, useState } from "react";
import { API_KEYS_CHANGED, api } from "../api";
import type { Permissions } from "../bindings/Permissions";
import type { Settings } from "../bindings/Settings";

const PERMISSION_POLL_MS = 2_000;

export function SetupChecklist({ settings }: { settings: Settings }) {
  const [permissions, setPermissions] = useState<Permissions | null>(null);
  const [hasTranscriptionKey, setHasTranscriptionKey] = useState<boolean | null>(null);

  const refresh = useCallback(() => {
    void api.getPermissions().then(setPermissions);
    void api.hasApiKey(settings.transcription.provider, "transcription").then(setHasTranscriptionKey);
  }, [settings.transcription.provider]);

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    window.addEventListener(API_KEYS_CHANGED, refresh);
    return () => {
      window.removeEventListener("focus", refresh);
      window.removeEventListener(API_KEYS_CHANGED, refresh);
    };
  }, [refresh]);

  useEffect(() => {
    if (permissions === null || (permissions.accessibility && permissions.microphone === "granted")) return;
    const timer = window.setInterval(refresh, PERMISSION_POLL_MS);
    return () => window.clearInterval(timer);
  }, [permissions, refresh]);

  if (permissions === null || hasTranscriptionKey === null) return null;
  const microphoneGranted = permissions.microphone === "granted";
  if (permissions.accessibility && microphoneGranted && hasTranscriptionKey) return null;

  return (
    <section className="setup" aria-label="Setup">
      <h2>Finish setting up</h2>
      <ol>
        <li className={permissions.accessibility ? "is-done" : undefined}>
          <div>
            <strong>Allow Accessibility access</strong>
            <p className="step-detail">Utter needs it to paste your words and to read the text around your cursor.</p>
          </div>
          {!permissions.accessibility && (
            <span className="inline">
              <button type="button" className="primary" onClick={() => void api.requestAccessibility()}>
                Allow…
              </button>
              <button type="button" className="quiet" onClick={() => void api.openPermissionSettings("accessibility")}>
                Open Settings
              </button>
            </span>
          )}
        </li>
        <li className={hasTranscriptionKey ? "is-done" : undefined}>
          <div>
            <strong>Add a speech-to-text API key</strong>
            <p className="step-detail">Paste it under Speech to text below. Groq has a free tier.</p>
          </div>
        </li>
        <li className={microphoneGranted ? "is-done" : undefined}>
          <div>
            <strong>Allow the microphone</strong>
            <p className="step-detail">
              {permissions.microphone === "denied"
                ? "Microphone access was turned off. Turn Utter on in System Settings."
                : "Utter only records while you dictate."}
            </p>
          </div>
          {permissions.microphone === "notDetermined" && (
            <button type="button" className="primary" onClick={() => void api.requestMicrophone().then(refresh)}>
              Allow…
            </button>
          )}
          {permissions.microphone === "denied" && (
            <button type="button" className="quiet" onClick={() => void api.openPermissionSettings("microphone")}>
              Open Settings
            </button>
          )}
        </li>
      </ol>
    </section>
  );
}
