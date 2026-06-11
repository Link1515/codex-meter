import { isTauri } from "@tauri-apps/api/core";
import { useEffect, useRef, type RefObject } from "react";
import { setWindowSize } from "./api";

export const MIN_WINDOW_WIDTH = 280;
export const MIN_WINDOW_HEIGHT = 232;
export const MAX_WINDOW_WIDTH = 420;
export const MAX_WINDOW_HEIGHT = 360;
const SIZE_CHANGE_THRESHOLD = 1;
const CONTENT_SIZE_PADDING = 8;
const SETTLE_DELAY_MS = [80, 240, 600];

type WindowSize = {
  width: number;
  height: number;
};

type ContentSizeMetrics = {
  scrollWidth: number;
  scrollHeight: number;
  boundingWidth: number;
  boundingHeight: number;
};

export function useAutoWindowSize(contentRef: RefObject<HTMLElement | null>): void {
  const lastAppliedSize = useRef<WindowSize | undefined>(undefined);

  useEffect(() => {
    const content = contentRef.current;
    if (!content || !isTauri()) {
      return;
    }

    let animationFrameId: number | undefined;
    const settleTimeoutIds: number[] = [];

    const applySize = () => {
      const nextSize = measureWindowSize(content);
      const previousSize = lastAppliedSize.current;

      if (previousSize && isSameSize(previousSize, nextSize)) {
        return;
      }

      void setWindowSize(nextSize.width, nextSize.height)
        .then(() => {
          lastAppliedSize.current = nextSize;
        })
        .catch(() => {
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
    window.addEventListener("resize", scheduleApplySize);
    scheduleApplySize();
    SETTLE_DELAY_MS.forEach((delayMs) => {
      settleTimeoutIds.push(window.setTimeout(scheduleApplySize, delayMs));
    });

    return () => {
      resizeObserver.disconnect();
      window.removeEventListener("resize", scheduleApplySize);
      settleTimeoutIds.forEach((timeoutId) => window.clearTimeout(timeoutId));
      if (animationFrameId !== undefined) {
        window.cancelAnimationFrame(animationFrameId);
      }
    };
  }, [contentRef]);
}

function measureWindowSize(content: HTMLElement): WindowSize {
  const compactProbe = createCompactProbe(content);
  document.body.appendChild(compactProbe);

  try {
    const rect = compactProbe.getBoundingClientRect();

    return resolveWindowSize({
      scrollWidth: compactProbe.scrollWidth,
      scrollHeight: compactProbe.scrollHeight,
      boundingWidth: rect.width,
      boundingHeight: rect.height
    });
  } finally {
    compactProbe.remove();
  }
}

function createCompactProbe(content: HTMLElement): HTMLElement {
  const probe = content.cloneNode(true) as HTMLElement;

  probe.style.position = "fixed";
  probe.style.left = "-10000px";
  probe.style.top = "0";
  probe.style.width = `${MIN_WINDOW_WIDTH}px`;
  probe.style.minWidth = `${MIN_WINDOW_WIDTH}px`;
  probe.style.maxWidth = `${MIN_WINDOW_WIDTH}px`;
  probe.style.height = "auto";
  probe.style.minHeight = "0";
  probe.style.maxHeight = "none";
  probe.style.visibility = "hidden";
  probe.style.pointerEvents = "none";
  probe.style.contain = "layout style";

  return probe;
}

export function resolveWindowSize(metrics: ContentSizeMetrics): WindowSize {
  const measuredWidth = safeMax(metrics.scrollWidth, metrics.boundingWidth);
  const measuredHeight = safeMax(metrics.scrollHeight, metrics.boundingHeight);
  const nextWidth = expandWhenNeeded(measuredWidth, MIN_WINDOW_WIDTH);
  const nextHeight = expandWhenNeeded(measuredHeight, MIN_WINDOW_HEIGHT);

  return {
    width: clamp(Math.ceil(nextWidth), MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH),
    height: clamp(Math.ceil(nextHeight), MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT)
  };
}

function expandWhenNeeded(measuredSize: number, compactSize: number): number {
  return measuredSize > compactSize ? measuredSize + CONTENT_SIZE_PADDING : compactSize;
}

function safeMax(...values: number[]): number {
  const finiteValues = values.filter(Number.isFinite);

  return finiteValues.length > 0 ? Math.max(...finiteValues) : 0;
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
