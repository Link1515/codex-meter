import { invoke } from "@tauri-apps/api/core";

export async function setAlwaysOnTop(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_always_on_top", { enabled });
}

export async function startDragging(): Promise<void> {
  return invoke<void>("start_dragging");
}
