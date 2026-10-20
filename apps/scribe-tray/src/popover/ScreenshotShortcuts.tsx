import { useEffect, useId, useRef } from "react";
import { LISTENING_MESSAGE, rowName } from "./shortcutCapture";
import type { ShortcutKind, ShortcutView } from "./types";
import { useScreenshotShortcuts } from "./useScreenshotShortcuts";

interface ScreenshotShortcutsProps {
  readonly onStatus: (message: string) => void;
}

interface ShortcutRowProps {
  readonly shortcut: ShortcutView;
  readonly isListening: boolean;
  readonly isLocked: boolean;
  readonly onChange: (kind: ShortcutKind) => void;
  readonly onReset: (kind: ShortcutKind) => void;
}

function ShortcutRow({
  shortcut,
  isListening,
  isLocked,
  onChange,
  onReset,
}: ShortcutRowProps) {
  const row = useRef<HTMLLIElement>(null);
  const name = rowName(shortcut.kind);

  useEffect(() => {
    if (isListening) {
      row.current?.focus();
    }
  }, [isListening]);

  return (
    <li className="shortcuts__row" ref={row} tabIndex={-1}>
      <span className="shortcuts__name">{name}</span>
      {isListening ? (
        <span className="shortcuts__listening">{LISTENING_MESSAGE}</span>
      ) : (
        <kbd className="shortcuts__keys">{shortcut.label}</kbd>
      )}
      <button
        className="actions__link"
        type="button"
        disabled={isLocked}
        aria-label={`Change ${name} shortcut, currently ${shortcut.label}`}
        onClick={() => onChange(shortcut.kind)}
      >
        Change
      </button>
      <button
        className="actions__link"
        type="button"
        disabled={isLocked}
        aria-label={`Reset to default for ${name} shortcut`}
        onClick={() => onReset(shortcut.kind)}
      >
        Reset to default
      </button>
    </li>
  );
}

export function ScreenshotShortcuts({ onStatus }: ScreenshotShortcutsProps) {
  const { shortcuts, listening, startListening, reset } =
    useScreenshotShortcuts(onStatus);
  const headingId = useId();

  return (
    <section className="shortcuts" aria-labelledby={headingId}>
      <h3 id={headingId} className="shortcuts__heading">
        Screenshot shortcuts
      </h3>
      <ul className="shortcuts__list">
        {shortcuts.map((shortcut) => (
          <ShortcutRow
            key={shortcut.kind}
            shortcut={shortcut}
            isListening={listening === shortcut.kind}
            isLocked={listening !== null}
            onChange={(kind) => void startListening(kind)}
            onReset={(kind) => void reset(kind)}
          />
        ))}
      </ul>
    </section>
  );
}
