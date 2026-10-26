import { useEffect, useId, useRef } from "react";
import type { SwitchPromptView } from "../types";

interface SwitchPromptCardProps {
  readonly prompt: SwitchPromptView;
  readonly onSwitch: () => void;
  readonly onKeep: () => void;
}

export function SwitchPromptCard({ prompt, onSwitch, onKeep }: SwitchPromptCardProps) {
  const heading = useRef<HTMLHeadingElement>(null);
  const headingId = useId();

  useEffect(() => {
    heading.current?.focus();
  }, []);

  return (
    <section className="switch-prompt" aria-labelledby={headingId}>
      <h2 id={headingId} className="switch-prompt__heading" ref={heading} tabIndex={-1}>
        Switch meeting?
      </h2>
      <p className="switch-prompt__text">
        {`Still recording ${prompt.from_subject}. Switch to ${prompt.to_subject}?`}
      </p>
      <div className="switch-prompt__actions">
        <button className="actions__primary" type="button" onClick={onSwitch}>
          Switch
        </button>
        <button className="actions__link" type="button" onClick={onKeep}>
          Keep recording
        </button>
      </div>
    </section>
  );
}
