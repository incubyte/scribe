import { useEffect, useRef, useState } from "react";
import { dismissIdleNotice } from "./commands";

const DISMISS_KEYS = ["Enter", "Escape"];

export function IdleNotice() {
  const okButton = useRef<HTMLButtonElement>(null);
  const [dontShowAgain, setDontShowAgain] = useState(false);

  useEffect(() => {
    okButton.current?.focus();
  }, []);

  const dismiss = () => void dismissIdleNotice(dontShowAgain);

  useEffect(() => {
    const dismissOnKey = (event: KeyboardEvent) => {
      if (DISMISS_KEYS.includes(event.key)) {
        event.preventDefault();
        void dismissIdleNotice(dontShowAgain);
      }
    };
    window.addEventListener("keydown", dismissOnKey);
    return () => window.removeEventListener("keydown", dismissOnKey);
  }, [dontShowAgain]);

  return (
    <main className="window">
      <h1 className="window__title">Copied to clipboard</h1>
      <p className="window__text">
        Scribe saves screenshots into a meeting&apos;s folder only while it&apos;s recording. This one is on
        your clipboard, ready to paste.
      </p>
      <label className="window__option">
        <input
          type="checkbox"
          checked={dontShowAgain}
          onChange={(event) => setDontShowAgain(event.target.checked)}
        />
        Don&apos;t show this again
      </label>
      <button ref={okButton} className="button" type="button" onClick={dismiss}>
        OK
      </button>
    </main>
  );
}
