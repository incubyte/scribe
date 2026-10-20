import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useAnnouncement } from "./useAnnouncement";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("announcing a message to screen readers", () => {
  it("starts silent", () => {
    const { result } = renderHook(() => useAnnouncement());

    expect(result.current.message).toBe("");
  });

  it("clears the previous message before the new one is read out", () => {
    const { result } = renderHook(() => useAnnouncement());
    act(() => {
      result.current.announce("Recording started");
      vi.runAllTimers();
    });

    act(() => result.current.announce("Recording stopped"));

    expect(result.current.message).toBe("");
    act(() => vi.runAllTimers());
    expect(result.current.message).toBe("Recording stopped");
  });

  it("reads only the latest of two announcements made in quick succession", () => {
    const { result } = renderHook(() => useAnnouncement());

    act(() => {
      result.current.announce("first");
      result.current.announce("second");
      vi.runAllTimers();
    });

    expect(result.current.message).toBe("second");
  });

  it("drops a pending announcement when the component goes away", () => {
    const clear = vi.spyOn(window, "clearTimeout");
    const { result, unmount } = renderHook(() => useAnnouncement());
    act(() => result.current.announce("late"));
    clear.mockClear();

    unmount();

    expect(clear).toHaveBeenCalledTimes(1);
  });
});
