import { useEffect, useState } from "react";
import { api, events } from "../api";
import type { Dictation } from "../bindings/Dictation";
import { useTauriEvent } from "../useTauriEvent";

export function LastDictation() {
  const [dictation, setDictation] = useState<Dictation | null>(null);

  useEffect(() => {
    void api.getLastDictation().then(setDictation);
  }, []);
  useTauriEvent(events.onDictation, setDictation);

  if (!dictation) {
    return <p className="empty">Your most recent dictation shows up here. It is kept in memory only.</p>;
  }

  const cleaned = dictation.text.trim() !== dictation.rawTranscript.trim();
  return (
    <div className="last-dictation">
      <blockquote>{dictation.text.trim() || "Nothing was typed."}</blockquote>
      {cleaned && (
        <p className="raw">
          <span>Heard:</span> {dictation.rawTranscript}
        </p>
      )}
      {dictation.cleanupError && (
        <p className="is-error">Cleanup failed, so the raw transcript was typed: {dictation.cleanupError}</p>
      )}
      <p className="timings">
        {dictation.audioSeconds.toFixed(1)} s of audio · speech-to-text {dictation.transcriptionMs} ms
        {dictation.cleanupMs !== null && ` · cleanup ${dictation.cleanupMs} ms`}
      </p>
    </div>
  );
}
