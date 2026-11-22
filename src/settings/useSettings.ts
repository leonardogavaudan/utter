import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { api, errorMessage, events } from "../api";
import type { Settings } from "../bindings/Settings";
import { useTauriEvent } from "../useTauriEvent";

function sameSettings(left: Settings | null, right: Settings | null): boolean {
  return JSON.stringify(left) === JSON.stringify(right);
}

export function useSettings() {
  const [saved, setSaved] = useState<Settings | null>(null);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  const savedRef = useRef<Settings | null>(null);

  const acceptSaved = useCallback((settings: Settings) => {
    savedRef.current = settings;
    setSaved(settings);
  }, []);

  useEffect(() => {
    api
      .getSettings()
      .then((settings) => {
        acceptSaved(settings);
        setDraft(settings);
      })
      .then(() => Promise.all([api.getShortcutError(), api.getSettingsError()]))
      .then(([shortcut, settingsFile]) => {
        setShortcutError(shortcut);
        setSettingsError(settingsFile);
      })
      .catch((error: unknown) => setLoadError(errorMessage(error)));
  }, [acceptSaved]);

  useTauriEvent(events.onSettingsChanged, (settings) => {
    const previous = savedRef.current;
    setDraft((current) => (current === null || sameSettings(current, previous) ? settings : current));
    acceptSaved(settings);
    void api.getShortcutError().then(setShortcutError);
  });
  useTauriEvent(events.onSettingsError, setSettingsError);

  const dirty = useMemo(() => saved !== null && draft !== null && !sameSettings(saved, draft), [saved, draft]);

  const update = useCallback((change: (settings: Settings) => Settings) => {
    setDraft((current) => (current ? change(current) : current));
    setSaveError(null);
  }, []);

  const save = useCallback(async () => {
    if (!draft) return;
    setSaving(true);
    try {
      const stored = await api.saveSettings(draft);
      acceptSaved(stored);
      setDraft(stored);
      setSaveError(null);
      setSettingsError(null);
      setShortcutError(await api.getShortcutError());
    } catch (error) {
      setSaveError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  }, [draft, acceptSaved]);

  const revert = useCallback(() => {
    setDraft(saved);
    setSaveError(null);
  }, [saved]);

  return {
    saved,
    draft,
    dirty,
    saving,
    loadError,
    saveError,
    shortcutError,
    settingsError,
    update,
    save,
    revert,
  };
}
