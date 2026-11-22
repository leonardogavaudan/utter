import { useCallback, useEffect, useState } from "react";
import { api, events } from "../api";
import type { Dictation } from "../bindings/Dictation";
import { useTauriEvent } from "../useTauriEvent";

const COPY_FEEDBACK_MS = 2_000;

type CopyState = "idle" | "copied" | "failed";

export function LastDictation() {
  const [dictation, setDictation] = useState<Dictation | null>(null);
  const [copyState, setCopyState] = useState<CopyState>("idle");

  const showDictation = useCallback((next: Dictation | null) => {
    setDictation(next);
    setCopyState("idle");
  }, []);

  useEffect(() => {
    void api.getLastDictation().then(showDictation);
  }, [showDictation]);
  useTauriEvent(events.onDictation, showDictation);

  useEffect(() => {
    if (copyState === "idle") return;
    const timer = window.setTimeout(() => setCopyState("idle"), COPY_FEEDBACK_MS);
    return () => window.clearTimeout(timer);
  }, [copyState]);

  if (!dictation) {
    return <p className="empty">Your most recent dictation shows up here. It is kept in memory only.</p>;
  }

  const text = dictation.text.trim();
  const cleaned = text !== dictation.rawTranscript.trim();
  const copy = () => {
    api.copyLastDictation().then(
      () => setCopyState("copied"),
      () => setCopyState("failed"),
    );
  };

  return (
    <div className="last-dictation">
      <blockquote>{text || "Nothing was typed."}</blockquote>
      {cleaned && (
        <p className="raw">
          <span>Heard:</span> {dictation.rawTranscript}
        </p>
      )}
      {dictation.cleanupError && (
        <p className="is-error">Cleanup failed, so the raw transcript was typed: {dictation.cleanupError}</p>
      )}
      <div className="last-dictation-footer">
        <p className="timings">
          {dictation.audioSeconds.toFixed(1)} s of audio · speech-to-text {dictation.transcriptionMs} ms
          {dictation.cleanupMs !== null && ` · cleanup ${dictation.cleanupMs} ms`}
        </p>
        {text && (
          <button type="button" className="quiet" onClick={copy}>
            {copyState === "copied" ? "Copied" : copyState === "failed" ? "Could not copy" : "Copy"}
          </button>
        )}
      </div>
    </div>
  );
}
