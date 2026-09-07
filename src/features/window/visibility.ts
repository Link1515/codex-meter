import { isTauri } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";
import { getWindowPollingAllowed } from "./api";

export const usageRefreshRequestedEvent = "codex-meter://usage-refresh-requested";
const windowVisibilityChangedEvent = "codex-meter://window-visibility-changed";

type WindowPollingEligibility = {
  isWindowPollingAllowed: boolean;
  setWindowPollingAllowed: (allowed: boolean) => void;
};

export function useWindowPollingEligibility(): WindowPollingEligibility {
  const [isWindowPollingAllowed, setWindowPollingAllowed] = useState(() => !isTauri());

  useEffect(() => {
    if (!isTauri()) {
      return;
    }

    let isActive = true;
    const unlistenFunctions: UnlistenFn[] = [];
    const refreshEligibility = () => {
      void getWindowPollingAllowed()
        .then((allowed) => {
          if (isActive) {
            setWindowPollingAllowed(allowed);
          }
        })
        .catch(() => {
          if (isActive) {
            setWindowPollingAllowed(false);
          }
        });
    };

    refreshEligibility();
    void listen<boolean>(windowVisibilityChangedEvent, (event) => {
      if (isActive) {
        setWindowPollingAllowed(event.payload);
      }
    }).then((unlisten) => {
      if (isActive) {
        unlistenFunctions.push(unlisten);
      } else {
        unlisten();
      }
    });
    void getCurrentWindow()
      .onFocusChanged((event) => {
        if (event.payload) {
          refreshEligibility();
        }
      })
      .then((unlisten) => {
        if (isActive) {
          unlistenFunctions.push(unlisten);
        } else {
          unlisten();
        }
      });

    return () => {
      isActive = false;
      unlistenFunctions.forEach((unlisten) => unlisten());
    };
  }, []);

  return { isWindowPollingAllowed, setWindowPollingAllowed };
}
