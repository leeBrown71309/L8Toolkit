import {
  ArrowSquareOut,
  Camera,
  Check,
  ClockCounterClockwise,
  DownloadSimple,
  File,
  LockSimple,
  TextT,
  Tray,
  X,
} from "@phosphor-icons/react";
import { useCallback, useEffect, useRef, useState, type KeyboardEvent, type PointerEvent } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { backend, describeError } from "../../shared/backend";
import type { RecentView } from "../../shared/types";
import { useShellIcon } from "../../shared/use-shell-icon";
import { isDragGesture, relativeAge } from "./recents-time";

import "./recents-panel.css";

const COPIED_FEEDBACK_MS = 1100;
/** One row of cards: up/down move like left/right. */
const STEP_KEYS: Record<string, number> = { ArrowLeft: -1, ArrowUp: -1, ArrowRight: 1, ArrowDown: 1 };
const ERROR_DURATION_MS = 3200;
/** Fake password length: the real length is never hinted. */
const MASK = "••••••••••";

interface RecentsPanelProps {
  items: RecentView[];
  isActive: boolean;
  /** Files are being dragged over the notch. */
  dropActive: boolean;
  onClose: (restoreFocus: boolean) => void;
  onEngaged: () => void;
  /** An item is being dragged out of the notch (its own drops are ignored). */
  onDragOut: () => void;
}

/**
 * The Recents tab: what was copied, captured or dropped on the notch lately.
 * Click copies an item again, dragging it drops it into any app.
 */
export function RecentsPanel({ items, isActive, dropActive, onClose, onEngaged, onDragOut }: RecentsPanelProps) {
  const { t } = useI18n();
  const rootRef = useRef<HTMLDivElement>(null);
  const stripRef = useRef<HTMLDivElement>(null);
  const [selectedIndex, setSelected] = useState(0);
  const [copiedId, setCopiedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Removing the last card must not leave the selection past the end.
  const selected = Math.max(0, Math.min(selectedIndex, items.length - 1));

  useEffect(() => {
    if (isActive) rootRef.current?.focus({ preventScroll: true });
  }, [isActive]);

  // A new item arrives on the left: selection and scroll go back there.
  const firstId = items[0]?.id;
  useEffect(() => {
    setSelected(0);
    stripRef.current?.scrollTo({ left: 0 });
  }, [firstId]);

  // The selected card stays in view, whether it moved by key or by click.
  useEffect(() => {
    stripRef.current?.children[selected]?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }, [selected]);

  // The strip only scrolls sideways: a plain mouse wheel scrolls it too
  // (no Shift needed). Native listener, React's wheel events are passive.
  const hasItems = items.length > 0;
  useEffect(() => {
    const strip = stripRef.current;
    if (!strip) return;
    const handleWheel = (event: WheelEvent) => {
      if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
      event.preventDefault();
      strip.scrollBy({ left: event.deltaY });
    };
    strip.addEventListener("wheel", handleWheel, { passive: false });
    return () => strip.removeEventListener("wheel", handleWheel);
  }, [hasItems]);

  useEffect(() => {
    if (!copiedId) return;
    const timer = window.setTimeout(() => setCopiedId(null), COPIED_FEEDBACK_MS);
    return () => window.clearTimeout(timer);
  }, [copiedId]);

  useEffect(() => {
    if (!error) return;
    const timer = window.setTimeout(() => setError(null), ERROR_DURATION_MS);
    return () => window.clearTimeout(timer);
  }, [error]);

  const run = useCallback(
    (action: Promise<void>, onDone?: () => void) => {
      onEngaged();
      action.then(onDone).catch((failure: unknown) => setError(describeError(failure)));
    },
    [onEngaged],
  );

  const copy = useCallback((id: string) => run(backend.recentsCopy(id), () => setCopiedId(id)), [run]);
  const open = useCallback((id: string) => run(backend.recentsOpen(id)), [run]);
  const remove = useCallback((id: string) => run(backend.recentsRemove(id)), [run]);
  const startDrag = useCallback(
    (id: string) => {
      onDragOut();
      run(backend.recentsDrag(id));
    },
    [onDragOut, run],
  );

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    onEngaged();
    if (event.key === "Escape") {
      event.preventDefault();
      onClose(true);
      return;
    }
    const current = items[selected];
    // Alt+arrows and Alt+digits switch widgets.
    if (event.altKey || !current) return;
    const step = STEP_KEYS[event.key];
    if (step !== undefined) {
      event.preventDefault();
      // From the latest value: key repeat can fire twice before a render.
      const last = items.length - 1;
      setSelected((index) => Math.max(0, Math.min(last, Math.min(index, last) + step)));
    } else if (event.key === "Home" || event.key === "End") {
      event.preventDefault();
      setSelected(event.key === "Home" ? 0 : items.length - 1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      if (event.ctrlKey) {
        if (current.kind !== "text") open(current.id);
      } else {
        copy(current.id);
      }
    } else if (event.key === "Delete") {
      event.preventDefault();
      remove(current.id);
    }
  };

  return (
    <div ref={rootRef} className="recents" tabIndex={-1} onKeyDown={handleKeyDown} data-drop={dropActive}>
      {/* Kept bare on purpose: the keyboard shortcuts are listed in Settings › Recents. */}
      {items.length > 0 && (
        <div className="recents__bar">
          <button type="button" className="recents__clear" onClick={() => run(backend.recentsClear())}>
            {t("recents.clear")}
          </button>
        </div>
      )}
      {items.length === 0 ? (
        <EmptyState />
      ) : (
        <div
          ref={stripRef}
          className="recents__strip"
          role="listbox"
          aria-label={t("recents.list")}
          aria-activedescendant={items[selected] ? `recent-${items[selected].id}` : undefined}
        >
          {items.map((item, index) => (
            <RecentCard
              key={item.id}
              item={item}
              selected={index === selected}
              copied={copiedId === item.id}
              onCopy={() => {
                setSelected(index);
                copy(item.id);
              }}
              onOpen={() => open(item.id)}
              onRemove={() => remove(item.id)}
              onDragStart={() => startDrag(item.id)}
            />
          ))}
        </div>
      )}
      {error && (
        <div className="recents__error" role="alert">
          {error}
        </div>
      )}
      <div className="recents__drop" aria-hidden={!dropActive}>
        <DownloadSimple size={22} weight="bold" />
        <div>
          <div className="recents__drop-title">{t("recents.drop.title")}</div>
          <div className="recents__drop-text">{t("recents.drop.text")}</div>
        </div>
      </div>
    </div>
  );
}

interface RecentCardProps {
  item: RecentView;
  selected: boolean;
  copied: boolean;
  onCopy: () => void;
  onOpen: () => void;
  onRemove: () => void;
  onDragStart: () => void;
}

function RecentCard({ item, selected, copied, onCopy, onOpen, onRemove, onDragStart }: RecentCardProps) {
  const { t } = useI18n();
  const press = useRef<{ x: number; y: number; dragged: boolean } | null>(null);
  const age = relativeAge(item.createdAtMs);
  const missing = item.kind === "files" && item.files.every((file) => !file.exists);

  // Pressing and moving a few pixels hands the item to a Windows drag, so it
  // can be dropped into any app; a press without movement is a click (copy).
  const handlePointerDown = (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0 || (event.target as HTMLElement).closest(".recent-card__action")) return;
    press.current = { x: event.clientX, y: event.clientY, dragged: false };
  };
  const handlePointerMove = (event: PointerEvent<HTMLDivElement>) => {
    const start = press.current;
    if (!start || start.dragged || missing || event.buttons !== 1) return;
    if (isDragGesture(start.x, start.y, event.clientX, event.clientY)) {
      start.dragged = true;
      onDragStart();
    }
  };
  const handlePointerUp = () => {
    const start = press.current;
    press.current = null;
    if (start && !start.dragged) onCopy();
  };

  const label = cardLabel(item, t);
  return (
    <div
      id={`recent-${item.id}`}
      className="recent-card"
      role="option"
      aria-selected={selected}
      aria-label={label}
      title={t("recents.cardHint")}
      data-kind={item.kind}
      data-missing={missing}
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerCancel={() => {
        press.current = null;
      }}
    >
      <div className="recent-card__body">
        <CardContent item={item} />
      </div>
      <div className="recent-card__meta">
        <OriginIcon item={item} />
        <span className="recent-card__meta-text">
          {item.sensitive && item.sourceApp ? item.sourceApp : t(age.key, { count: String(age.count) })}
        </span>
      </div>
      <div className="recent-card__actions">
        {item.kind !== "text" && !missing && (
          <button type="button" className="recent-card__action" aria-label={t("recents.open")} title={`${t("recents.open")} (Ctrl+Enter)`} onClick={onOpen}>
            <ArrowSquareOut size={13} weight="bold" />
          </button>
        )}
        <button type="button" className="recent-card__action" aria-label={t("recents.remove")} title={`${t("recents.remove")} (${t("keys.delete")})`} onClick={onRemove}>
          <X size={13} weight="bold" />
        </button>
      </div>
      <div className="recent-card__copied" data-visible={copied} aria-hidden={!copied}>
        <Check size={18} weight="bold" />
        {t("recents.copied")}
      </div>
    </div>
  );
}

function cardLabel(item: RecentView, t: ReturnType<typeof useI18n>["t"]): string {
  if (item.sensitive) return t("recents.hidden");
  if (item.kind === "text") return item.preview ?? "";
  if (item.kind === "image") return t(item.origin === "screenshot" ? "recents.origin.screenshot" : "recents.image");
  return item.files.map((file) => file.name).join(", ");
}

function CardContent({ item }: { item: RecentView }) {
  const { t } = useI18n();
  if (item.kind === "image") {
    return item.thumbnail ? (
      <img className="recent-card__image" src={item.thumbnail} alt="" draggable={false} />
    ) : (
      <Camera size={24} />
    );
  }
  if (item.kind === "files") return <FilesContent item={item} />;
  if (item.sensitive) {
    return (
      <div className="recent-card__secret">
        <LockSimple size={16} weight="fill" />
        <span className="recent-card__mask">{MASK}</span>
        <span className="recent-card__secret-label">{t("recents.hidden")}</span>
      </div>
    );
  }
  return <p className="recent-card__text">{item.preview}</p>;
}

function FilesContent({ item }: { item: RecentView }) {
  const { t } = useI18n();
  const first = item.files[0];
  const icon = useShellIcon(first?.exists ? first.path : null);
  const extra = item.files.length - 1;
  const name = first?.exists === false ? t("recents.missing") : first?.name;
  return (
    <div className="recent-card__files">
      <span className="recent-card__file-icon">
        {icon ? <img src={icon} width={28} height={28} alt="" draggable={false} /> : <File size={24} />}
        {extra > 0 && <span className="recent-card__file-count">+{extra}</span>}
      </span>
      <span className="recent-card__file-name" title={first?.name}>
        {name}
      </span>
    </div>
  );
}

function OriginIcon({ item }: { item: RecentView }) {
  if (item.sensitive) return <LockSimple size={11} weight="bold" />;
  if (item.origin === "screenshot") return <Camera size={11} weight="bold" />;
  if (item.origin === "shelf") return <Tray size={11} weight="bold" />;
  if (item.kind === "text") return <TextT size={11} weight="bold" />;
  return <ClockCounterClockwise size={11} weight="bold" />;
}

function EmptyState() {
  const { t } = useI18n();
  return (
    <div className="recents-empty">
      <span className="recents-empty__icon">
        <ClockCounterClockwise size={20} />
      </span>
      <div>
        <div className="recents-empty__title">{t("recents.empty.title")}</div>
        <p className="recents-empty__text">{t("recents.empty.text")}</p>
      </div>
    </div>
  );
}
