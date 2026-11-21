import { useEffect, useState } from "react";
import { api, events } from "../api";
import { useTauriEvent } from "../useTauriEvent";

export function UpdateBanner() {
  const [version, setVersion] = useState<string | null>(null);

  useEffect(() => {
    void api.getPendingUpdate().then(setVersion);
  }, []);
  useTauriEvent(events.onUpdateReady, setVersion);

  if (!version) return null;
  return (
    <section className="update-banner" role="status">
      <span>Utter {version} is installed. Restart to start using it.</span>
      <button type="button" className="primary" onClick={() => void api.restartToUpdate()}>
        Restart
      </button>
    </section>
  );
}
