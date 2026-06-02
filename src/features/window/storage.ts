import type { WindowPinState } from "./types";

const pinKey = "codex-meter:pin-state";

type PinStorage = Pick<Storage, "getItem" | "setItem">;

export function loadPinState(storage: PinStorage = localStorage): WindowPinState {
  const raw = storage.getItem(pinKey);
  if (!raw) {
    return { isPinned: false, updatedAt: new Date().toISOString() };
  }

  try {
    const parsed = JSON.parse(raw) as WindowPinState;
    return {
      isPinned: Boolean(parsed.isPinned),
      updatedAt: parsed.updatedAt || new Date().toISOString()
    };
  } catch {
    return { isPinned: false, updatedAt: new Date().toISOString() };
  }
}

export function savePinState(state: WindowPinState, storage: PinStorage = localStorage): void {
  storage.setItem(pinKey, JSON.stringify(state));
}
