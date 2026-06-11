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

export async function showWindow(): Promise<void> {
  return invoke<void>("show_window");
}

export async function setWindowSize(width: number, height: number, show = false): Promise<void> {
  return invoke<void>("set_window_size", { width, height, show });
}

export async function getWindowPlacement(): Promise<WindowPlacementState> {
  return invoke<WindowPlacementState>("get_window_placement");
}

export async function restoreWindowPlacement(placement: WindowPlacementState): Promise<WindowPlacementState> {
  return invoke<WindowPlacementState>("restore_window_placement", { placement });
}
