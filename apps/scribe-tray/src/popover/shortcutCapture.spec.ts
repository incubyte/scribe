import { describe, expect, it } from "vitest";
import {
  IN_USE_MESSAGE,
  describeRebind,
  isModifierOnly,
  pressedKeysFrom,
} from "./shortcutCapture";

function keydown(init: KeyboardEventInit) {
  return new KeyboardEvent("keydown", init);
}

describe("capturing a shortcut from a key press", () => {
  it("reports the physical key even when Option changes the typed character", () => {
    const pressed = pressedKeysFrom(
      keydown({ code: "Digit2", key: "™", altKey: true, ctrlKey: true }),
    );

    expect(pressed).toEqual({
      code: "Digit2",
      control: true,
      alt: true,
      shift: false,
      meta: false,
    });
  });

  it("reports every modifier that is held", () => {
    const pressed = pressedKeysFrom(
      keydown({ code: "KeyR", shiftKey: true, metaKey: true }),
    );

    expect(pressed).toMatchObject({ code: "KeyR", shift: true, meta: true });
  });

  it("ignores a press of a modifier on its own", () => {
    expect(pressedKeysFrom(keydown({ code: "ShiftLeft", shiftKey: true }))).toBeUndefined();
    expect(isModifierOnly("MetaRight")).toBe(true);
    expect(isModifierOnly("KeyR")).toBe(false);
  });
});

describe("describing the outcome of a rebind", () => {
  it("names the combination that was accepted", () => {
    expect(describeRebind({ outcome: "accepted", label: "⌃R" }, "region")).toBe(
      "Capture region is now ⌃R",
    );
  });

  it("explains what a combination needs in the words the app supplies", () => {
    const message = "Use Ctrl or Alt with a letter, number or function key";

    expect(describeRebind({ outcome: "needs_modifier", message }, "region")).toBe(message);
  });

  it("names the row that already uses the combination", () => {
    expect(
      describeRebind({ outcome: "duplicate_of", kind: "region" }, "full_screen"),
    ).toBe("Already used for Capture region");
    expect(
      describeRebind({ outcome: "duplicate_of", kind: "full_screen" }, "region"),
    ).toBe("Already used for Capture full screen");
  });

  it("says another app is using the combination", () => {
    expect(describeRebind({ outcome: "in_use_by_another_app" }, "region")).toBe(
      IN_USE_MESSAGE,
    );
    expect(IN_USE_MESSAGE).toBe("Another app is using this shortcut");
  });
});
