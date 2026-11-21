import type { UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef } from "react";

export function useTauriEvent<T>(
  subscribe: (handler: (payload: T) => void) => Promise<UnlistenFn>,
  handler: (payload: T) => void,
) {
  const latestHandler = useRef(handler);
  latestHandler.current = handler;

  useEffect(() => {
    const unlisten = subscribe((payload) => latestHandler.current(payload));
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, [subscribe]);
}
