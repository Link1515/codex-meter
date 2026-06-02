import { invoke } from "@tauri-apps/api/core";
import type { WindowPlacementState } from "./types";

export async function setAlwaysOnTop(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_always_on_top", { enabled });
}

export async function getAlwaysOnTop(): Promise<boolean> {
  return invoke<boolean>("get_always_on_top");
}

export async function startDragging(): Promise<void> {
  return invoke<void>("start_dragging");
}

export async function getWindowPlacement(): Promise<WindowPlacementState> {
  return invoke<WindowPlacementState>("get_window_placement");
}

export async function restoreWindowPlacement(placement: WindowPlacementState): Promise<WindowPlacementState> {
  return invoke<WindowPlacementState>("restore_window_placement", { placement });
}
