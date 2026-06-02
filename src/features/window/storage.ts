import type { WindowPinState, WindowPlacementState } from "./types";

const pinKey = "codex-meter:pin-state";
const placementKey = "codex-meter:window-placement";

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

export function loadWindowPlacement(storage: PinStorage = localStorage): WindowPlacementState | undefined {
  const raw = storage.getItem(placementKey);
  if (!raw) {
    return undefined;
  }

  try {
    const parsed = JSON.parse(raw) as Partial<WindowPlacementState>;
    if (!isValidPlacement(parsed)) {
      return undefined;
    }

    return {
      x: parsed.x,
      y: parsed.y,
      width: parsed.width,
      height: parsed.height,
      displayId: typeof parsed.displayId === "string" ? parsed.displayId : undefined,
      updatedAt: typeof parsed.updatedAt === "string" ? parsed.updatedAt : new Date().toISOString()
    };
  } catch {
    return undefined;
  }
}

export function saveWindowPlacement(state: WindowPlacementState, storage: PinStorage = localStorage): void {
  if (isValidPlacement(state)) {
    storage.setItem(placementKey, JSON.stringify(state));
  }
}

function isValidPlacement(value: Partial<WindowPlacementState>): value is WindowPlacementState {
  return (
    typeof value.x === "number" &&
    Number.isFinite(value.x) &&
    typeof value.y === "number" &&
    Number.isFinite(value.y) &&
    typeof value.width === "number" &&
    Number.isFinite(value.width) &&
    typeof value.height === "number" &&
    Number.isFinite(value.height) &&
    value.width > 0 &&
    value.height > 0
  );
}
