import { describe, expect, it, vi } from "vitest";
import {
  loadPinState,
  loadWindowPlacement,
  savePinState,
  saveWindowPlacement
} from "../../src/features/window/storage";

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

  it("persists valid window placement", () => {
    const storage = {
      getItem: vi.fn(() => null),
      setItem: vi.fn()
    };
    const placement = {
      x: 10,
      y: 20,
      width: 300,
      height: 255,
      updatedAt: "2026-06-01T00:00:00.000Z"
    };

    saveWindowPlacement(placement, storage);

    expect(storage.setItem).toHaveBeenCalledWith("codex-meter:window-placement", JSON.stringify(placement));
  });

  it("ignores invalid stored window placement", () => {
    const storage = {
      getItem: vi.fn(() => JSON.stringify({ x: 10, y: 20, width: 0, height: 255 })),
      setItem: vi.fn()
    };

    expect(loadWindowPlacement(storage)).toBeUndefined();
  });
});
