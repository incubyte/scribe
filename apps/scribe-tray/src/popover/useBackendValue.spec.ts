import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { act, cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useBackendValue } from "./useBackendValue";

const COUNT_EVENT = "count://changed";

const isCount = (payload: unknown): payload is number => typeof payload === "number";

function deferredFetch() {
  let resolve: ((value: number) => void) | undefined;
  const fetchValue = vi.fn(
    () =>
      new Promise<number>((settle) => {
        resolve = settle;
      }),
  );
  return { fetchValue, resolveWith: (value: number) => resolve?.(value) };
}

afterEach(async () => {
  cleanup();
  await new Promise((resolve) => setTimeout(resolve));
  clearMocks();
});

describe("a value kept in step with the backend", () => {
  it("shows what the fetch returns", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const { fetchValue, resolveWith } = deferredFetch();
    const { result } = renderHook(() => useBackendValue(fetchValue, COUNT_EVENT, isCount));

    await act(async () => resolveWith(1));

    expect(result.current.value).toBe(1);
  });

  it("keeps a newer event's value when an older fetch resolves after it", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const { fetchValue, resolveWith } = deferredFetch();
    const { result } = renderHook(() => useBackendValue(fetchValue, COUNT_EVENT, isCount));
    await waitFor(() => expect(fetchValue).toHaveBeenCalledTimes(1));

    await act(() => emit(COUNT_EVENT, 2));
    await act(async () => resolveWith(1));

    expect(result.current.value).toBe(2);
  });

  it("takes a fetch started after the latest event", async () => {
    mockIPC(() => undefined, { shouldMockEvents: true });
    const { fetchValue, resolveWith } = deferredFetch();
    const { result } = renderHook(() => useBackendValue(fetchValue, COUNT_EVENT, isCount));
    await act(() => emit(COUNT_EVENT, 2));

    act(() => window.dispatchEvent(new Event("focus")));
    await act(async () => resolveWith(3));

    expect(result.current.value).toBe(3);
  });
});
