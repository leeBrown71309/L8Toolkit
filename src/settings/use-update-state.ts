import { useCallback, useEffect, useState } from "react";

import { backend, BACKEND_EVENTS, describeError } from "../shared/backend";
import type { UpdateState } from "../shared/types";
import { useBackendEvent } from "../shared/use-backend-event";

export interface UpdateController {
  state: UpdateState;
  check: () => void;
  install: () => void;
}

/**
 * Update status shared by the navigation badge and the Updates section.
 * Rust owns the state (it also checks on its own in the background); this
 * hook mirrors it and forwards the two user actions.
 */
export function useUpdateState(): UpdateController {
  const [state, setState] = useState<UpdateState>({ status: "idle" });

  useEffect(() => {
    let alive = true;
    backend
      .updateState()
      .then((current) => {
        if (alive) setState(current);
      })
      .catch((error: unknown) => console.error("Cannot read the update state", error));
    return () => {
      alive = false;
    };
  }, []);

  useBackendEvent<UpdateState>(BACKEND_EVENTS.updateState, setState);

  const check = useCallback(() => {
    backend
      .updateCheck()
      .then(setState)
      .catch((error: unknown) => setState({ status: "failed", message: describeError(error) }));
  }, []);

  const install = useCallback(() => {
    // Progress arrives through events; only a failure needs handling here.
    backend.updateInstall().catch((error: unknown) => setState({ status: "failed", message: describeError(error) }));
  }, []);

  return { state, check, install };
}
