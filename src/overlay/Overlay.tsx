import { useCallback, useEffect, useState } from "react";
import { api, events } from "../api";
import type { CopyReason } from "../bindings/CopyReason";
import type { Notice } from "../bindings/Notice";
import type { Phase } from "../bindings/Phase";
import { useTauriEvent } from "../useTauriEvent";

type View = { kind: "recording" } | { kind: "processing" } | { kind: "notice"; notice: Notice } | { kind: "hidden" };

export function Overlay() {
  const [view, setView] = useState<View>({ kind: "hidden" });

  const onPhase = useCallback((phase: Phase) => {
    if (phase === "recording") setView({ kind: "recording" });
    if (phase === "processing") setView({ kind: "processing" });
  }, []);
  const onNotice = useCallback((notice: Notice) => setView({ kind: "notice", notice }), []);

  useTauriEvent(events.onPhase, onPhase);
  useTauriEvent(events.onNotice, onNotice);

  useEffect(() => {
    void api.getPhase().then(onPhase);
  }, [onPhase]);

  switch (view.kind) {
    case "hidden":
      return null;
    case "recording":
      return (
        <div className="pill" role="status">
          <span className="recording-dot" />
          <span className="bars" aria-hidden="true">
            <i />
            <i />
            <i />
            <i />
          </span>
          Listening
        </div>
      );
    case "processing":
      return (
        <div className="pill" role="status">
          <span className="spinner" aria-hidden="true" />
          Transcribing
        </div>
      );
    case "notice":
      return (
        <div className={`pill${view.notice.kind === "failed" ? " is-error" : ""}`} role="alert">
          {describeNotice(view.notice)}
        </div>
      );
  }
}

function describeNotice(notice: Notice): string {
  switch (notice.kind) {
    case "noSpeech":
      return "Didn't catch anything";
    case "copied":
      return describeCopyReason(notice.reason);
    case "cleanupSkipped":
      return "Typed without cleanup, see Settings";
    case "failed":
      return notice.message;
  }
}

function describeCopyReason(reason: CopyReason): string {
  switch (reason) {
    case "accessibilityMissing":
      return "Copied. Allow Accessibility to paste";
    case "appChanged":
      return "Copied to clipboard, the app changed";
    case "pasteFailed":
      return "Copied. Press ⌘V to paste";
    case "noTextField":
      return "No text field found, so it's on your clipboard";
  }
}
