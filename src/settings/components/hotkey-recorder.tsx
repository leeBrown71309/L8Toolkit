import { useState, type KeyboardEvent } from "react";

import { useI18n } from "../../i18n/i18n-context";
import { acceleratorToKeycaps, recordShortcut } from "../../shared/accelerator";

interface HotkeyRecorderProps {
  value: string;
  disabled?: boolean;
  onChange: (accelerator: string) => void;
}

/** Click, press the new combination; Escape cancels. */
export function HotkeyRecorder({ value, disabled = false, onChange }: HotkeyRecorderProps) {
  const { t } = useI18n();
  const [recording, setRecording] = useState(false);
  const [hint, setHint] = useState<string | null>(null);

  const stopRecording = () => {
    setRecording(false);
    setHint(null);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLButtonElement>) => {
    if (!recording) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === "Escape") {
      stopRecording();
      return;
    }
    const recorded = recordShortcut(event.nativeEvent);
    if (recorded.status === "invalid") setHint(t(recorded.reason === "needsModifier" ? "hotkey.needsModifier" : "hotkey.unsupportedKey"));
    if (recorded.status === "complete") {
      stopRecording();
      if (recorded.accelerator !== value) onChange(recorded.accelerator);
    }
  };

  return (
    <div className="hotkey">
      <button
        type="button"
        className="hotkey__field"
        data-recording={recording}
        disabled={disabled}
        aria-label={recording ? t("hotkey.recordingLabel") : t("hotkey.label", { shortcut: value })}
        onClick={() => setRecording((current) => !current)}
        onKeyDown={handleKeyDown}
        onBlur={stopRecording}
      >
        {recording ? (
          <span className="hotkey__prompt">{hint ?? t("hotkey.prompt")}</span>
        ) : (
          acceleratorToKeycaps(value).map((key) => (
            <kbd key={key} className="hotkey__key">
              {key}
            </kbd>
          ))
        )}
      </button>
    </div>
  );
}
