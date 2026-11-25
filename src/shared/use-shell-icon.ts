import { useEffect, useState } from "react";

import { backend } from "./backend";

/** Bounded so a long session of searching cannot grow memory forever. */
const MAX_CACHED_ICONS = 600;

const resolvedIcons = new Map<string, string | null>();
const pendingIcons = new Map<string, Promise<string | null>>();

function rememberIcon(target: string, icon: string | null): void {
  resolvedIcons.set(target, icon);
  if (resolvedIcons.size > MAX_CACHED_ICONS) {
    const oldest = resolvedIcons.keys().next().value;
    if (oldest !== undefined) resolvedIcons.delete(oldest);
  }
}

function loadIcon(target: string): Promise<string | null> {
  const pending = pendingIcons.get(target);
  if (pending) return pending;
  const request = backend
    .resolveIcon(target)
    .catch((error: unknown) => {
      console.warn(`No icon for ${target}`, error);
      return null;
    })
    .then((icon) => {
      rememberIcon(target, icon);
      pendingIcons.delete(target);
      return icon;
    });
  pendingIcons.set(target, request);
  return request;
}

/** Real app/file icon as a data URL, or null while loading / unavailable. */
export function useShellIcon(target: string | null): string | null {
  const [icon, setIcon] = useState<string | null>(() => (target ? resolvedIcons.get(target) ?? null : null));

  useEffect(() => {
    if (!target) {
      setIcon(null);
      return;
    }
    const cached = resolvedIcons.get(target);
    if (cached !== undefined) {
      setIcon(cached);
      return;
    }
    let alive = true;
    loadIcon(target).then((loaded) => {
      if (alive) setIcon(loaded);
    });
    return () => {
      alive = false;
    };
  }, [target]);

  return icon;
}
