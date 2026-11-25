import { useEffect, useRef, useState } from "react";

import { backend, describeError } from "../../../shared/backend";
import type { SearchResult, SearchSection } from "../../../shared/types";

interface SearchState {
  sections: SearchSection[];
  error: string | null;
}

/**
 * Searches on every keystroke (the Rust index answers in a few ms) and keeps
 * the previous results on screen until the next ones arrive, so the list
 * never flashes empty while typing. Bump `reloadToken` to search again after
 * an action changed the results (pin, delete).
 */
export function useSearch(query: string, reloadToken: number): SearchState {
  const [state, setState] = useState<SearchState>({ sections: [], error: null });
  const latestRequest = useRef(0);

  useEffect(() => {
    const requestId = ++latestRequest.current;
    if (!query.trim()) {
      setState({ sections: [], error: null });
      return;
    }
    backend
      .search(query)
      .then((sections) => {
        if (requestId === latestRequest.current) setState({ sections, error: null });
      })
      .catch((error: unknown) => {
        if (requestId === latestRequest.current) setState({ sections: [], error: describeError(error) });
      });
  }, [query, reloadToken]);

  return state;
}

/** Pins and most launched items, refreshed every time the notch opens. */
export function useFrequentItems(active: boolean, reloadToken: number): SearchResult[] {
  const [items, setItems] = useState<SearchResult[]>([]);

  useEffect(() => {
    if (!active) return;
    let alive = true;
    backend
      .frequentItems()
      .then((results) => {
        if (alive) setItems(results);
      })
      .catch((error: unknown) => console.error("Cannot load frequent items", error));
    return () => {
      alive = false;
    };
  }, [active, reloadToken]);

  return items;
}
