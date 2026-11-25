import { MusicNotesSimple } from "@phosphor-icons/react";
import { useState, type SyntheticEvent } from "react";

import "./cover-art.css";

interface CoverArtProps {
  /** Data URL of the cover, or null when the player gave none. */
  src: string | null;
  iconSize: number;
  onLoad?: (image: HTMLImageElement) => void;
}

/**
 * The cover of what is playing, or a placeholder sleeve when the player has
 * no cover or sends one the webview cannot decode (a broken image icon would
 * look like a bug, a designed sleeve looks intentional).
 */
export function CoverArt({ src, iconSize, onLoad }: CoverArtProps) {
  const [failedSource, setFailedSource] = useState<string | null>(null);

  if (!src || src === failedSource) {
    return (
      <span className="cover-placeholder" aria-hidden="true">
        <MusicNotesSimple size={iconSize} weight="fill" />
      </span>
    );
  }

  return (
    <img
      key={src}
      src={src}
      alt=""
      draggable={false}
      onLoad={(event: SyntheticEvent<HTMLImageElement>) => onLoad?.(event.currentTarget)}
      onError={() => setFailedSource(src)}
    />
  );
}
