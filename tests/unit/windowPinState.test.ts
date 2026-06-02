import { describe, expect, it, vi } from "vitest";
import { loadPinState, savePinState } from "../../src/features/window/storage";

describe("window pin state storage", () => {
  it("loads a safe default when storage is empty", () => {
    const storage = {
      getItem: vi.fn(() => null),
      setItem: vi.fn()
    };

    expect(loadPinState(storage).isPinned).toBe(false);
  });

  it("persists pinned state", () => {
    const storage = {
      getItem: vi.fn(() => null),
      setItem: vi.fn()
    };
    const state = { isPinned: true, updatedAt: "2026-06-01T00:00:00.000Z" };

    savePinState(state, storage);

    expect(storage.setItem).toHaveBeenCalledWith("codex-meter:pin-state", JSON.stringify(state));
  });
});
