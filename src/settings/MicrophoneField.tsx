import { useCallback, useEffect, useState } from "react";
import { api } from "../api";
import type { Microphone } from "../bindings/Microphone";
import { Field } from "./Layout";

export function MicrophoneField({ value, onChange }: { value: string | null; onChange: (id: string | null) => void }) {
  const [microphones, setMicrophones] = useState<Microphone[]>([]);

  const refresh = useCallback(() => {
    void api.listMicrophones().then(setMicrophones);
  }, []);

  useEffect(() => {
    refresh();
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [refresh]);

  const unavailable =
    value !== null && microphones.length > 0 && !microphones.some((microphone) => microphone.id === value);

  return (
    <Field
      label="Microphone"
      hint="Automatic records with your Mac's built-in microphone when the default one is a Bluetooth headset, so the music in your headphones keeps its quality."
    >
      {(control) => (
        <select {...control} value={value ?? ""} onChange={(event) => onChange(event.target.value || null)}>
          <option value="">Automatic</option>
          {microphones.map((microphone) => (
            <option key={microphone.id} value={microphone.id}>
              {microphone.name}
              {microphone.connection === "bluetooth" ? " (Bluetooth, lowers headphone sound quality)" : ""}
            </option>
          ))}
          {unavailable && <option value={value}>Disconnected microphone, using Automatic</option>}
        </select>
      )}
    </Field>
  );
}
