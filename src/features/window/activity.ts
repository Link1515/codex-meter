import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export type WindowActivitySnapshot = {
  isActive: boolean;
};

export type WindowActivityListener = (snapshot: WindowActivitySnapshot) => void;

export type UnsubscribeWindowActivity = () => void;

export async function getWindowActivity(): Promise<WindowActivitySnapshot> {
  if (!isTauri()) {
    return { isActive: document.visibilityState === "visible" };
  }

  const appWindow = getCurrentWindow();
  const [isFocused, isVisible, isMinimized] = await Promise.all([
    appWindow.isFocused(),
    appWindow.isVisible(),
    appWindow.isMinimized()
  ]);

  return {
    isActive: isFocused && isVisible && !isMinimized && document.visibilityState === "visible"
  };
}

export async function subscribeWindowActivity(listener: WindowActivityListener): Promise<UnsubscribeWindowActivity> {
  let disposed = false;
  let lastIsActive: boolean | undefined;
  const unlisteners: UnsubscribeWindowActivity[] = [];

  const notify = async (): Promise<void> => {
    try {
      const snapshot = await getWindowActivity();
      if (disposed || snapshot.isActive === lastIsActive) {
        return;
      }

      lastIsActive = snapshot.isActive;
      listener(snapshot);
    } catch {
      if (!disposed && lastIsActive !== false) {
        lastIsActive = false;
        listener({ isActive: false });
      }
    }
  };

  const handleVisibilityChange = (): void => {
    void notify();
  };

  document.addEventListener("visibilitychange", handleVisibilityChange);
  unlisteners.push(() => document.removeEventListener("visibilitychange", handleVisibilityChange));

  if (isTauri()) {
    const unlistenFocus = await getCurrentWindow().onFocusChanged(() => {
      void notify();
    });
    unlisteners.push(unlistenFocus);
  } else {
    window.addEventListener("focus", handleVisibilityChange);
    window.addEventListener("blur", handleVisibilityChange);
    unlisteners.push(() => {
      window.removeEventListener("focus", handleVisibilityChange);
      window.removeEventListener("blur", handleVisibilityChange);
    });
  }

  void notify();

  return () => {
    disposed = true;
    for (const unlisten of unlisteners) {
      unlisten();
    }
  };
}
