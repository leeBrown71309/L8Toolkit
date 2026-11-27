import { useEffect, useState } from "react";

import { backend, BACKEND_EVENTS } from "../../shared/backend";
import type { RecentView } from "../../shared/types";
import { useBackendEvent } from "../../shared/use-backend-event";

/** The Recents list, kept in sync with recents/mod.rs. */
export function useRecents(enabled: boolean): RecentView[] {
  const [items, setItems] = useState<RecentView[]>([]);

  useEffect(() => {
    if (!enabled) {
      setItems([]);
      return;
    }
    let alive = true;
    backend
      .recentsList()
      .then((list) => {
        if (alive) setItems(list);
      })
      .catch((error: unknown) => console.error("Cannot read Recents", error));
    return () => {
      alive = false;
    };
  }, [enabled]);

  useBackendEvent<RecentView[]>(BACKEND_EVENTS.recentsChanged, (next) => {
    if (enabled) setItems(next);
  });

  return enabled ? items : [];
}
