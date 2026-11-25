import type { NowPlaying } from "../../shared/types";
import { AudioBars } from "./audio-bars";
import { CoverArt } from "./cover-art";
import { Equalizer } from "./equalizer";

import "./now-playing-live.css";

export type LiveMode = "idle" | "peek" | "open";

interface NowPlayingLiveProps {
  media: NowPlaying;
  mode: LiveMode;
  /** Audio bars across the folded notch (setting). */
  showAudioBars: boolean;
}

/**
 * The "live activity" drawn inside the black notch while media plays:
 * audio bars across the folded notch, cover and title when it peeks at a new
 * track. Every part is positioned from the notch geometry, so it rides the
 * morph.
 */
export function NowPlayingLive({ media, mode, showAudioBars }: NowPlayingLiveProps) {
  return (
    <div className="live" data-mode={mode} aria-hidden={mode === "open"}>
      {showAudioBars && (
        <span className="live__bars">
          <AudioBars active={mode === "idle" && media.playing} />
        </span>
      )}
      <span className="live__artwork">
        <CoverArt src={media.artwork} iconSize={18} />
      </span>
      <span className="live__text">
        <span className="live__title">{media.title}</span>
        <span className="live__artist">{media.artist || media.sourceName}</span>
      </span>
      <Equalizer playing={media.playing} className="live__equalizer" />
    </div>
  );
}
