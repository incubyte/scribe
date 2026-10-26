import { type RefObject, useId, useLayoutEffect, useRef } from "react";
import type { SwitchPromptView } from "../types";

interface SwitchPromptCardProps {
  readonly prompt: SwitchPromptView;
  readonly onSwitch: () => void;
  readonly onKeep: () => void;
  readonly fallbackFocus: RefObject<HTMLElement | null>;
}

export function SwitchPromptCard({ prompt, onSwitch, onKeep, fallbackFocus }: SwitchPromptCardProps) {
  const card = useRef<HTMLElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const headingId = useId();
  useFocusWhileOpen(card, heading, fallbackFocus);
  return (
    <section className="switch-prompt" aria-labelledby={headingId} ref={card}>
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

function useFocusWhileOpen(
  card: RefObject<HTMLElement | null>,
  heading: RefObject<HTMLHeadingElement | null>,
  fallbackFocus: RefObject<HTMLElement | null>,
) {
  useLayoutEffect(() => {
    const focusedBefore = document.activeElement;
    const cardElement = card.current;
    heading.current?.focus();
    return () => returnFocus(cardElement, focusedBefore, fallbackFocus);
  }, [card, heading, fallbackFocus]);
}

function returnFocus(
  card: HTMLElement | null,
  focusedBefore: Element | null,
  fallbackFocus: RefObject<HTMLElement | null>,
) {
  if (!isFocusLeftWith(card)) {
    return;
  }
  const target = canTakeFocusBack(focusedBefore) ? focusedBefore : fallbackFocus.current;
  target?.focus();
}

function isFocusLeftWith(card: HTMLElement | null): boolean {
  const focused = document.activeElement;
  return focused === null || focused === document.body || card?.contains(focused) === true;
}

function canTakeFocusBack(element: Element | null): element is HTMLElement {
  return element instanceof HTMLElement && element !== document.body && element.isConnected;
}
