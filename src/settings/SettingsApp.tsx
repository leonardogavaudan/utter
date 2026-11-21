import { useEffect, useState } from "react";
import appIcon from "../../src-tauri/icons/128x128@2x.png";
import { api, events } from "../api";
import type { ContextMode } from "../bindings/ContextMode";
import type { Phase } from "../bindings/Phase";
import type { ProviderPreset } from "../bindings/ProviderPreset";
import type { TriggerMode } from "../bindings/TriggerMode";
import { useTauriEvent } from "../useTauriEvent";
import { LastDictation } from "./LastDictation";
import { Field, Section, Segmented, Switch } from "./Layout";
import { ProviderFields } from "./ProviderFields";
import { SetupChecklist } from "./SetupChecklist";
import { Keys, ShortcutRecorder } from "./ShortcutRecorder";
import { useSettings } from "./useSettings";

const PHASE_LABELS: Record<Phase, string> = {
  idle: "Ready",
  recording: "Listening",
  processing: "Transcribing",
};

const CONTEXT_OPTIONS: { value: ContextMode; title: string; description: string }[] = [
  {
    value: "screen",
    title: "Everything visible in the window",
    description:
      "Best for spelling names and terms. Reads the text on screen in the app you are typing in, nearest your cursor first (up to 2,000 characters), plus the text around your cursor.",
  },
  {
    value: "cursor",
    title: "Text around the cursor",
    description: "Reads up to 400 characters before and 200 after the cursor, plus any selected text.",
  },
  {
    value: "appOnly",
    title: "App and window title",
    description: "Lets cleanup match the tone, for example casual in a chat app and formal in an email.",
  },
  {
    value: "off",
    title: "Off",
    description: "Only the transcript and your vocabulary are sent to the cleanup model.",
  },
];

export function SettingsApp() {
  const { saved, draft, dirty, saving, loadError, saveError, shortcutError, update, save, revert } = useSettings();
  const [providers, setProviders] = useState<ProviderPreset[]>([]);
  const [phase, setPhase] = useState<Phase>("idle");

  useEffect(() => {
    void api.listProviders().then(setProviders);
    void api.getPhase().then(setPhase);
  }, []);
  useTauriEvent(events.onPhase, setPhase);

  if (loadError) return <p className="load-error">Utter could not load its settings: {loadError}</p>;
  if (!draft || !saved || providers.length === 0) return null;

  const triggerVerb = saved.trigger === "hold" ? "Hold" : "Press";

  return (
    <div className="app">
      <header className="app-header">
        <img src={appIcon} alt="" width={44} height={44} />
        <div className="app-title">
          <h1>Utter</h1>
          {shortcutError ? (
            <p className="is-error">The dictation shortcut is not working. Choose another one below.</p>
          ) : (
            <p>
              {triggerVerb} <Keys shortcut={saved.shortcut} /> to dictate
              {saved.trigger === "toggle" && ", then press again to finish"}.
            </p>
          )}
        </div>
        <span className={`status status-${phase}`}>{PHASE_LABELS[phase]}</span>
      </header>

      <main>
        <SetupChecklist settings={saved} />

        <Section title="Shortcut">
          <Field
            label="Dictation shortcut"
            hint={shortcutError && <span className="is-error">{shortcutError}. Another app may already use it.</span>}
          >
            {(control) => (
              <ShortcutRecorder
                control={control}
                value={draft.shortcut}
                onChange={(shortcut) => update((settings) => ({ ...settings, shortcut }))}
              />
            )}
          </Field>
          <Field label="Mode" hint="Escape cancels a dictation without typing anything.">
            {(control) => (
              <Segmented<TriggerMode>
                control={control}
                value={draft.trigger}
                options={[
                  { value: "hold", label: "Hold to talk" },
                  { value: "toggle", label: "Press to start and stop" },
                ]}
                onChange={(trigger) => update((settings) => ({ ...settings, trigger }))}
              />
            )}
          </Field>
        </Section>

        <Section title="Speech to text" description="Your recording is sent here when you stop talking.">
          <ProviderFields
            providerRole="transcription"
            providers={providers}
            value={draft.transcription}
            onChange={(endpoint) =>
              update((settings) => ({ ...settings, transcription: { ...settings.transcription, ...endpoint } }))
            }
          />
          <Field label="Language" hint="Optional code such as en or fr. Leave empty to detect it automatically.">
            {(control) => (
              <input
                {...control}
                value={draft.transcription.language ?? ""}
                placeholder="Auto-detect"
                spellCheck={false}
                onChange={(event) =>
                  update((settings) => ({
                    ...settings,
                    transcription: { ...settings.transcription, language: event.target.value || null },
                  }))
                }
              />
            )}
          </Field>
        </Section>

        <Section
          title="Cleanup"
          description="A language model fixes punctuation, removes filler words, and spells names using the context below."
          action={
            <Switch
              label="Clean up dictations"
              checked={draft.cleanup.enabled}
              onChange={(enabled) => update((settings) => ({ ...settings, cleanup: { ...settings.cleanup, enabled } }))}
            />
          }
        >
          {draft.cleanup.enabled ? (
            <>
              <ProviderFields
                providerRole="cleanup"
                providers={providers}
                value={draft.cleanup}
                onChange={(endpoint) =>
                  update((settings) => ({ ...settings, cleanup: { ...settings.cleanup, ...endpoint } }))
                }
              />
              <Field
                label="Instructions"
                hint="Optional. For example: use British spelling, or never use exclamation marks."
              >
                {(control) => (
                  <textarea
                    {...control}
                    rows={3}
                    value={draft.cleanup.instructions}
                    onChange={(event) =>
                      update((settings) => ({
                        ...settings,
                        cleanup: { ...settings.cleanup, instructions: event.target.value },
                      }))
                    }
                  />
                )}
              </Field>
            </>
          ) : (
            <p className="empty">Utter types the raw transcript exactly as the speech-to-text model returns it.</p>
          )}
        </Section>

        <Section
          title="Context"
          description="What Utter reads from the app you are typing in, to help the cleanup model."
        >
          <fieldset className="choices">
            <legend className="visually-hidden">Context</legend>
            {CONTEXT_OPTIONS.map((option) => (
              <label key={option.value} className={`choice${draft.context === option.value ? " is-selected" : ""}`}>
                <input
                  type="radio"
                  name="context"
                  value={option.value}
                  checked={draft.context === option.value}
                  onChange={() => update((settings) => ({ ...settings, context: option.value }))}
                />
                <span>
                  <strong>{option.title}</strong>
                  <span>{option.description}</span>
                </span>
              </label>
            ))}
          </fieldset>
          <p className="privacy">
            Utter has no server. Audio goes only to your speech-to-text provider, and text and context go only to your
            cleanup provider. Context is read through macOS Accessibility, never from screenshots, and password fields
            are never read. Nothing is saved except these settings, and API keys stay in your Keychain.
          </p>
        </Section>

        <Section
          title="Vocabulary"
          description="Names and terms you use often. They are passed to both models to get the spelling right."
        >
          <textarea
            className="vocabulary"
            rows={5}
            placeholder={"One per line, for example:\nPriyanka\nKubernetes"}
            aria-label="Vocabulary, one term per line"
            value={draft.vocabulary.join("\n")}
            onChange={(event) => update((settings) => ({ ...settings, vocabulary: event.target.value.split("\n") }))}
          />
        </Section>

        <Section title="Last dictation">
          <LastDictation />
        </Section>
      </main>

      {(dirty || saveError) && (
        <footer className="save-bar" role="status">
          <span className={saveError ? "is-error" : undefined}>{saveError ?? "You have unsaved changes."}</span>
          <span className="inline">
            <button type="button" className="quiet" onClick={revert} disabled={saving}>
              Revert
            </button>
            <button type="button" className="primary" onClick={() => void save()} disabled={saving || !dirty}>
              {saving ? "Saving…" : "Save"}
            </button>
          </span>
        </footer>
      )}
    </div>
  );
}
