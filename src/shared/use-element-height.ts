import { useLayoutEffect, type RefObject } from "react";

/** Reports the element's border-box height whenever it changes. */
export function useElementHeight(ref: RefObject<HTMLElement | null>, onChange: (height: number) => void): void {
  useLayoutEffect(() => {
    const element = ref.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      const height = entry?.borderBoxSize[0]?.blockSize ?? element.offsetHeight;
      onChange(Math.ceil(height));
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref, onChange]);
}
