import { useEffect, useState } from "react";
import { captureShortcut, shortcutKeys } from "../shortcut";
import type { ControlProps } from "./Layout";

export function Keys({ shortcut }: { shortcut: string }) {
  return (
    <span className="keys">
      {shortcutKeys(shortcut).map((key) => (
        <kbd key={key}>{key}</kbd>
      ))}
    </span>
  );
}

export function ShortcutRecorder({
  value,
  onChange,
  control,
}: {
  value: string;
  onChange: (shortcut: string) => void;
  control: ControlProps;
}) {
  const [recording, setRecording] = useState(false);
  const [hint, setHint] = useState<string | null>(null);

  useEffect(() => {
    if (!recording) return;
    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
      const capture = captureShortcut(event);
      switch (capture.kind) {
        case "pending":
          return;
        case "invalid":
          setHint(capture.reason);
          return;
        case "cancelled":
          setHint(null);
          setRecording(false);
          return;
        case "captured":
          setHint(null);
          setRecording(false);
          onChange(capture.shortcut);
      }
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recording, onChange]);

  return (
    <div className="shortcut">
      <button
        type="button"
        id={control.id}
        aria-labelledby={`${control["aria-labelledby"]} ${control.id}`}
        aria-describedby={`${control.id}-status`}
        className={`shortcut-button${recording ? " is-recording" : ""}`}
        onClick={() => {
          setHint(null);
          setRecording((current) => !current);
        }}
        onBlur={() => setRecording(false)}
      >
        {recording ? <span className="shortcut-prompt">Press the new shortcut…</span> : <Keys shortcut={value} />}
      </button>
      <span className="field-hint" id={`${control.id}-status`} aria-live="polite">
        {hint ?? (recording ? "Press Escape to keep the current shortcut." : "Click to change.")}
      </span>
    </div>
  );
}
