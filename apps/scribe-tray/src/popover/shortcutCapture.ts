import type { PressedKeys, RebindOutcome, ShortcutKind } from "./types";

const MODIFIER_CODES: readonly string[] = [
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
];

const ROW_NAMES: Readonly<Record<ShortcutKind, string>> = {
  full_screen: "Capture full screen",
  region: "Capture region",
};

export const NEEDS_MODIFIER_MESSAGE =
  "Use ⌘, ⌃ or ⌥ with a letter, number or function key";
export const IN_USE_MESSAGE = "Another app is using this shortcut";
export const SHORTCUT_FAILURE_MESSAGE = "The shortcut could not be changed.";
export const SHORTCUTS_UNAVAILABLE_MESSAGE =
  "Scribe could not read the screenshot shortcuts.";

export const rowName = (kind: ShortcutKind) => ROW_NAMES[kind];

export function isModifierOnly(code: string): boolean {
  return MODIFIER_CODES.includes(code);
}

export function pressedKeysFrom(event: KeyboardEvent): PressedKeys | undefined {
  if (isModifierOnly(event.code)) {
    return undefined;
  }
  return {
    code: event.code,
    control: event.ctrlKey,
    alt: event.altKey,
    shift: event.shiftKey,
    meta: event.metaKey,
  };
}

export function describeRebind(outcome: RebindOutcome, kind: ShortcutKind): string {
  switch (outcome.outcome) {
    case "accepted":
      return `${rowName(kind)} is now ${outcome.label}`;
    case "needs_modifier":
      return NEEDS_MODIFIER_MESSAGE;
    case "duplicate_of":
      return `Already used for ${rowName(outcome.kind)}`;
    case "in_use_by_another_app":
      return IN_USE_MESSAGE;
  }
}
