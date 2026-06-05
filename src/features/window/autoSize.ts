import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { useEffect, useRef, type RefObject } from "react";

export const MIN_WINDOW_WIDTH = 280;
export const MIN_WINDOW_HEIGHT = 232;
export const MAX_WINDOW_WIDTH = 420;
export const MAX_WINDOW_HEIGHT = 360;
const SIZE_CHANGE_THRESHOLD = 1;

type WindowSize = {
  width: number;
  height: number;
};

type AutoWindowSizeOptions = {
  preferCompactSize?: boolean;
};

export function useAutoWindowSize(
  contentRef: RefObject<HTMLElement | null>,
  options: AutoWindowSizeOptions = {}
): void {
  const lastAppliedSize = useRef<WindowSize | undefined>(undefined);
  const preferCompactSize = options.preferCompactSize ?? false;

  useEffect(() => {
    const content = contentRef.current;
    if (!content || !isTauri()) {
      return;
    }

    const appWindow = getCurrentWindow();
    let animationFrameId: number | undefined;

    const applySize = () => {
      const nextSize = preferCompactSize ? compactWindowSize() : measureWindowSize(content);
      const previousSize = lastAppliedSize.current;

      if (previousSize && isSameSize(previousSize, nextSize)) {
        return;
      }

      lastAppliedSize.current = nextSize;
      void appWindow.setSize(new LogicalSize(nextSize.width, nextSize.height)).catch(() => {
        // Auto sizing is best effort; window command failures should not block usage refresh.
      });
    };

    const scheduleApplySize = () => {
      if (animationFrameId !== undefined) {
        window.cancelAnimationFrame(animationFrameId);
      }

      animationFrameId = window.requestAnimationFrame(applySize);
    };

    const resizeObserver = new ResizeObserver(scheduleApplySize);
    resizeObserver.observe(content);
    scheduleApplySize();

    return () => {
      resizeObserver.disconnect();
      if (animationFrameId !== undefined) {
        window.cancelAnimationFrame(animationFrameId);
      }
    };
  }, [contentRef, preferCompactSize]);
}

function measureWindowSize(content: HTMLElement): WindowSize {
  const nextWidth = content.scrollWidth > content.clientWidth ? content.scrollWidth : MIN_WINDOW_WIDTH;
  const nextHeight = content.scrollHeight > content.clientHeight ? content.scrollHeight : MIN_WINDOW_HEIGHT;

  return {
    width: clamp(Math.ceil(nextWidth), MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH),
    height: clamp(Math.ceil(nextHeight), MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT)
  };
}

function compactWindowSize(): WindowSize {
  return {
    width: MIN_WINDOW_WIDTH,
    height: MIN_WINDOW_HEIGHT
  };
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function isSameSize(previousSize: WindowSize, nextSize: WindowSize): boolean {
  return (
    Math.abs(previousSize.width - nextSize.width) <= SIZE_CHANGE_THRESHOLD &&
    Math.abs(previousSize.height - nextSize.height) <= SIZE_CHANGE_THRESHOLD
  );
}
