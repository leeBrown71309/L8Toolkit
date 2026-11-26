import { MusicNotesSimple, Pause, Play, SkipBack, SkipForward, WarningCircle } from "@phosphor-icons/react";
import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { backend, describeError } from "../../shared/backend";
import type { NowPlaying, PlaybackRequest } from "../../shared/types";
import { useShellIcon } from "../../shared/use-shell-icon";
import { CoverArt } from "./cover-art";
import { Equalizer } from "./equalizer";
import { ProgressBar } from "./progress-bar";
import { currentPositionMs } from "./use-now-playing";

import "../shared/transport-button.css";
import "./music-panel.css";

const ERROR_DURATION_MS = 3200;
const KEYBOARD_SEEK_STEP_MS = 10_000;
/** Covers wider than this are video thumbnails (YouTube...). */
const WIDE_ARTWORK_RATIO = 1.3;

interface MusicPanelProps {
  media: NowPlaying | null;
  isActive: boolean;
  /** Blur the cover behind the panel for an ambient glow. */
  ambient: boolean;
  onClose: (restoreFocus: boolean) => void;
  onEngaged: () => void;
}

export function MusicPanel({ media, isActive, ambient, onClose, onEngaged }: MusicPanelProps) {
  const { t } = useI18n();
  const rootRef = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (isActive) rootRef.current?.focus({ preventScroll: true });
  }, [isActive]);

  useEffect(() => {
    if (!error) return;
    const timer = window.setTimeout(() => setError(null), ERROR_DURATION_MS);
    return () => window.clearTimeout(timer);
  }, [error]);

  const send = useCallback(
    (request: PlaybackRequest) => {
      onEngaged();
      backend.mediaCommand(request).catch((commandError: unknown) => setError(describeError(commandError)));
    },
    [onEngaged],
  );

  const openSource = useCallback(() => {
    backend.mediaOpenSource().catch((openError: unknown) => setError(describeError(openError)));
  }, []);

  // Media keys inside the panel: Space plays/pauses, arrows seek, Ctrl+arrows skip.
  // Alt+arrows are left alone: they switch widgets.
  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    onEngaged();
    if (event.key === "Escape") {
      event.preventDefault();
      onClose(true);
      return;
    }
    if (!media || event.altKey) return;
    if (event.key === " " && media.canToggle) {
      event.preventDefault();
      send({ type: "togglePlayPause" });
    } else if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
      event.preventDefault();
      const forward = event.key === "ArrowRight";
      if (event.ctrlKey) {
        if (forward && media.canNext) send({ type: "next" });
        if (!forward && media.canPrevious) send({ type: "previous" });
      } else if (media.canSeek) {
        const step = forward ? KEYBOARD_SEEK_STEP_MS : -KEYBOARD_SEEK_STEP_MS;
        send({ type: "seek", positionMs: Math.round(Math.max(0, currentPositionMs(media) + step)) });
      }
    }
  };

  return (
    <div ref={rootRef} className="music" tabIndex={-1} onKeyDown={handleKeyDown} aria-label={t("music.nowPlaying")}>
      {media ? (
        <>
          {ambient && media.artwork && (
            <div key={media.artwork} className="music__ambient" style={{ backgroundImage: `url("${media.artwork}")` }} aria-hidden="true" />
          )}
          <div className="music__layout">
            <Artwork media={media} onOpen={openSource} />
            <div className="music__details">
              <div className="music__heading">
                <div className="music__title" title={media.title}>
                  {media.title}
                </div>
                <SourceBadge media={media} onOpen={openSource} />
              </div>
              <div className="music__artist">{[media.artist, media.album].filter(Boolean).join(" · ") || media.sourceName}</div>
              <ProgressBar media={media} onSeek={(positionMs) => send({ type: "seek", positionMs })}>
                <Controls media={media} onCommand={send} />
              </ProgressBar>
            </div>
          </div>
          {error && (
            <div className="music__error" role="alert">
              <WarningCircle size={14} weight="fill" />
              {error}
            </div>
          )}
        </>
      ) : (
        <EmptyState />
      )}
    </div>
  );
}

function Artwork({ media, onOpen }: { media: NowPlaying; onOpen: () => void }) {
  const { t } = useI18n();
  // Remembered per cover, so a new track never inherits the previous shape.
  const [wideSource, setWideSource] = useState<string | null>(null);
  const wide = media.artwork !== null && wideSource === media.artwork;
  const label = t("music.openPlayer", { player: media.sourceName });
  return (
    <button
      type="button"
      className="music__artwork"
      data-wide={wide}
      data-playing={media.playing}
      disabled={!media.sourceLaunchTarget}
      aria-label={label}
      title={media.sourceLaunchTarget ? label : undefined}
      onClick={onOpen}
    >
      <CoverArt
        src={media.artwork}
        iconSize={30}
        onLoad={(image) => {
          const isWide = image.naturalWidth / Math.max(1, image.naturalHeight) > WIDE_ARTWORK_RATIO;
          setWideSource(isWide ? media.artwork : null);
        }}
      />
    </button>
  );
}

function SourceBadge({ media, onOpen }: { media: NowPlaying; onOpen: () => void }) {
  const { t } = useI18n();
  const icon = useShellIcon(media.sourceLaunchTarget);
  return (
    <button
      type="button"
      className="music__source"
      disabled={!media.sourceLaunchTarget}
      title={media.sourceLaunchTarget ? t("music.openPlayer", { player: media.sourceName }) : media.sourceName}
      onClick={onOpen}
    >
      {icon ? <img src={icon} width={14} height={14} alt="" draggable={false} /> : <MusicNotesSimple size={13} />}
      <span className="music__source-name">{media.sourceName}</span>
      <Equalizer playing={media.playing} className="music__source-equalizer" />
    </button>
  );
}

function Controls({ media, onCommand }: { media: NowPlaying; onCommand: (request: PlaybackRequest) => void }) {
  const { t } = useI18n();
  return (
    <div className="music__controls">
      <button
        type="button"
        className="transport-button"
        aria-label={t("music.previous")}
        title={`${t("music.previous")} (Ctrl+←)`}
        disabled={!media.canPrevious}
        onClick={() => onCommand({ type: "previous" })}
      >
        <SkipBack size={16} weight="fill" />
      </button>
      <button
        type="button"
        className="transport-button transport-button--primary"
        aria-label={media.playing ? t("music.pause") : t("music.play")}
        title={`${media.playing ? t("music.pause") : t("music.play")} (${t("keys.space")})`}
        disabled={!media.canToggle}
        onClick={() => onCommand({ type: "togglePlayPause" })}
      >
        {/* Both icons stay mounted so the swap can cross-fade. */}
        <Pause className="transport-button__icon" data-visible={media.playing} size={17} weight="fill" />
        <Play className="transport-button__icon" data-visible={!media.playing} size={17} weight="fill" />
      </button>
      <button
        type="button"
        className="transport-button"
        aria-label={t("music.next")}
        title={`${t("music.next")} (Ctrl+→)`}
        disabled={!media.canNext}
        onClick={() => onCommand({ type: "next" })}
      >
        <SkipForward size={16} weight="fill" />
      </button>
    </div>
  );
}

function EmptyState() {
  const { t } = useI18n();
  return (
    <div className="music-empty">
      <span className="music-empty__icon">
        <MusicNotesSimple size={20} />
      </span>
      <div>
        <div className="music-empty__title">{t("music.empty.title")}</div>
        <p className="music-empty__text">{t("music.empty.text")}</p>
      </div>
    </div>
  );
}
