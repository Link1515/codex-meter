import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useRef, type RefObject } from "react";
import { revealWidgetWindow, setWindowSize } from "./api";

export const MIN_WINDOW_WIDTH = 270;
// Used only when the DOM has not produced a measurable layout yet.
export const MIN_WINDOW_HEIGHT = 190;
export const MAX_WINDOW_WIDTH = 420;
export const MAX_WINDOW_HEIGHT = 360;
const SIZE_CHANGE_THRESHOLD = 1;
const MAX_POST_RESIZE_VALIDATIONS = 2;

type WindowSize = {
  width: number;
  height: number;
};

type ContentSizeMetrics = {
  scrollWidth: number;
  scrollHeight: number;
  boundingWidth: number;
  boundingHeight: number;
  visualWidth?: number;
  visualHeight?: number;
};

type VisualExtent = {
  visualWidth: number;
  visualHeight: number;
};

export function useAutoWindowSize(contentRef: RefObject<HTMLElement | null>): void {
  const lastRequestedSize = useRef<WindowSize | undefined>(undefined);
  const hasRevealedWindow = useRef(false);

  useEffect(() => {
    const content = contentRef.current;
    if (!content || !isTauri()) {
      return;
    }

    let animationFrameId: number | undefined;
    let isResizeInFlight = false;
    let shouldRetryAfterResize = false;
    let postResizeValidationCount = 0;
    let isActive = true;
    let removeScaleChangeListener: (() => void) | undefined;

    const revealInitialWindow = () => {
      if (hasRevealedWindow.current) {
        return;
      }

      hasRevealedWindow.current = true;
      void revealWidgetWindow().catch(() => {
        // A failed reveal must not prevent later sizing or tray actions.
      });
    };

    const applySize = () => {
      if (isResizeInFlight) {
        shouldRetryAfterResize = true;
        return;
      }

      const nextSize = measureWindowSize(content);
      const matchesLastRequest =
        lastRequestedSize.current !== undefined && isSameSize(lastRequestedSize.current, nextSize);

      if (matchesLastRequest && contentFitsViewport(content)) {
        postResizeValidationCount = 0;
        revealInitialWindow();
        return;
      }

      isResizeInFlight = true;
      void setWindowSize({
        ...nextSize,
        viewportWidth: window.innerWidth,
        viewportHeight: window.innerHeight
      })
        .then(() => {
          lastRequestedSize.current = nextSize;
          window.requestAnimationFrame(() => {
            if (!isActive) {
              return;
            }

            if (contentFitsViewport(content)) {
              postResizeValidationCount = 0;
              revealInitialWindow();
              return;
            }

            if (postResizeValidationCount < MAX_POST_RESIZE_VALIDATIONS) {
              postResizeValidationCount += 1;
              scheduleApplySize();
              return;
            }

            // Content can be larger than the configured window maximum. Do
            // not loop native resize commands forever in that situation.
            revealInitialWindow();
          });
        })
        .catch(() => {
          // Keep the widget reachable if native sizing fails.
          revealInitialWindow();
        })
        .finally(() => {
          isResizeInFlight = false;
          if (shouldRetryAfterResize) {
            shouldRetryAfterResize = false;
            scheduleApplySize(true);
          }
        });
    };

    const scheduleApplySize = (restartValidation = false) => {
      if (restartValidation) {
        postResizeValidationCount = 0;
      }

      if (animationFrameId !== undefined) {
        window.cancelAnimationFrame(animationFrameId);
      }

      animationFrameId = window.requestAnimationFrame(applySize);
    };

    const handleBrowserResize = () => scheduleApplySize(true);
    const resizeObserver = new ResizeObserver(() => scheduleApplySize(true));
    resizeObserver.observe(content);
    window.addEventListener("resize", handleBrowserResize);
    scheduleApplySize(true);

    void document.fonts?.ready.then(() => {
      if (isActive) {
        scheduleApplySize(true);
      }
    });
    void getCurrentWindow()
      .onScaleChanged(() => scheduleApplySize(true))
      .then((unlisten) => {
        if (isActive) {
          removeScaleChangeListener = unlisten;
        } else {
          unlisten();
        }
      });

    return () => {
      isActive = false;
      resizeObserver.disconnect();
      window.removeEventListener("resize", handleBrowserResize);
      removeScaleChangeListener?.();
      if (animationFrameId !== undefined) {
        window.cancelAnimationFrame(animationFrameId);
      }
    };
  }, [contentRef]);
}

function measureWindowSize(content: HTMLElement): WindowSize {
  const probe = createContentMeasurementProbe(content);
  document.body.appendChild(probe);

  try {
    const rect = probe.getBoundingClientRect();

    return resolveWindowSize({
      scrollWidth: probe.scrollWidth,
      scrollHeight: probe.scrollHeight,
      boundingWidth: rect.width,
      boundingHeight: rect.height,
      ...measureVisualExtent(probe, rect)
    });
  } finally {
    probe.remove();
  }
}

/**
 * Measure against the compact baseline instead of the live window. The live
 * content fills its parent (`width: 100%`), so using it directly would retain
 * an oversized startup viewport even when the rendered meter needs less room.
 */
function createContentMeasurementProbe(content: HTMLElement): HTMLElement {
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

function contentFitsViewport(content: HTMLElement): boolean {
  const rect = content.getBoundingClientRect();
  const extent = measureVisualExtent(content, rect);

  return extent.visualWidth <= window.innerWidth && extent.visualHeight <= window.innerHeight;
}

function measureVisualExtent(content: HTMLElement, contentRect: DOMRect): VisualExtent {
  let visualRight = contentRect.right;
  let visualBottom = contentRect.bottom;

  content.querySelectorAll<HTMLElement>("*").forEach((element) => {
    const rect = element.getBoundingClientRect();
    visualRight = Math.max(visualRight, rect.right);
    visualBottom = Math.max(visualBottom, rect.bottom);
  });

  return {
    visualWidth: visualRight - contentRect.left,
    visualHeight: visualBottom - contentRect.top
  };
}

export function resolveWindowSize(metrics: ContentSizeMetrics): WindowSize {
  const measuredWidth = safeMax(metrics.scrollWidth, metrics.boundingWidth, metrics.visualWidth);
  const measuredHeight = safeMax(metrics.scrollHeight, metrics.boundingHeight, metrics.visualHeight);

  return {
    width: clamp(Math.ceil(measuredWidth), MIN_WINDOW_WIDTH, MAX_WINDOW_WIDTH),
    height: clamp(Math.ceil(measuredHeight), MIN_WINDOW_HEIGHT, MAX_WINDOW_HEIGHT)
  };
}

function safeMax(...values: Array<number | undefined>): number {
  const finiteValues = values.filter((value): value is number => Number.isFinite(value));

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
