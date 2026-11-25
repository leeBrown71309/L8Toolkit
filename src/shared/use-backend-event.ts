import { useEffect, useRef } from "react";

import { backend, type Unlisten } from "./backend";

/** Subscribes to a backend event for the component's lifetime. */
export function useBackendEvent<T>(event: string, handler: (payload: T) => void): void {
  const handlerRef = useRef(handler);

  useEffect(() => {
    handlerRef.current = handler;
  });

  useEffect(() => {
    let disposed = false;
    let unlisten: Unlisten | undefined;
    backend
      .listen<T>(event, (payload) => handlerRef.current(payload))
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch((error: unknown) => console.error(`Cannot listen to ${event}`, error));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [event]);
}
