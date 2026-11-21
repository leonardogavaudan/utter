import { useCallback, useEffect, useMemo, useState } from "react";
import { api, errorMessage } from "../api";
import type { Settings } from "../bindings/Settings";

export function useSettings() {
  const [saved, setSaved] = useState<Settings | null>(null);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);
  const [shortcutError, setShortcutError] = useState<string | null>(null);

  useEffect(() => {
    api
      .getSettings()
      .then((settings) => {
        setSaved(settings);
        setDraft(settings);
      })
      .then(() => api.getShortcutError())
      .then(setShortcutError)
      .catch((error: unknown) => setLoadError(errorMessage(error)));
  }, []);

  const dirty = useMemo(
    () => saved !== null && draft !== null && JSON.stringify(saved) !== JSON.stringify(draft),
    [saved, draft],
  );

  const update = useCallback((change: (settings: Settings) => Settings) => {
    setDraft((current) => (current ? change(current) : current));
    setSaveError(null);
  }, []);

  const save = useCallback(async () => {
    if (!draft) return;
    setSaving(true);
    try {
      const stored = await api.saveSettings(draft);
      setSaved(stored);
      setDraft(stored);
      setSaveError(null);
      setShortcutError(await api.getShortcutError());
    } catch (error) {
      setSaveError(errorMessage(error));
    } finally {
      setSaving(false);
    }
  }, [draft]);

  const revert = useCallback(() => {
    setDraft(saved);
    setSaveError(null);
  }, [saved]);

  return { saved, draft, dirty, saving, loadError, saveError, shortcutError, update, save, revert };
}
