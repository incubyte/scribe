import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it, vi } from "vitest";
import { cancelRegion, completeRegion, fetchRegionContext, fetchRegionFrame } from "./commands";

afterEach(() => clearMocks());

function replyingTo(command: string, reply: unknown) {
  const handler = vi.fn((received: string) => (received === command ? reply : undefined));
  mockIPC(handler);
  return handler;
}

describe("the region selector talks to the app", () => {
  it.each([
    ["region context", fetchRegionContext, "region_context"],
    ["region frame", fetchRegionFrame, "region_frame"],
    ["region completion", () => completeRegion({ x: 0, y: 0, width: 9, height: 9 }), "complete_region"],
  ])("refuses an unexpected %s from the app", async (what, ask, command) => {
    replyingTo(command, 42);

    await expect(ask()).rejects.toThrow(`Unexpected ${what} from Scribe`);
  });

  it("sends the chosen area when completing a region", async () => {
    const handler = replyingTo("complete_region", { outcome: "captured" });
    const area = { x: 1, y: 2, width: 30, height: 40 };

    await completeRegion(area);

    expect(handler).toHaveBeenCalledWith("complete_region", { area });
  });

  it("tells the app when the selection is cancelled", async () => {
    const handler = replyingTo("cancel_region", null);

    await cancelRegion();

    expect(handler).toHaveBeenCalledWith("cancel_region", {});
  });
});
