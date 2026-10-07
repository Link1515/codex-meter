import { describe, expect, it } from "vitest";
import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
  measureVisualExtent,
  resolveWindowSize
} from "../../src/features/window/autoSize";

describe("window auto size", () => {
  it("preserves the meter width in settings even when descendants report a larger extent", () => {
    expect(resolveWindowSize({
      scrollWidth: 420, scrollHeight: 205,
      boundingWidth: 270, boundingHeight: 205,
      visualWidth: 10000, visualHeight: 205
    }, 270)).toEqual({ width: 270, height: 205 });
  });

  it("preserves a wider meter width instead of resetting it to the minimum", () => {
    expect(resolveWindowSize({
      scrollWidth: 300, scrollHeight: 190,
      boundingWidth: 300, boundingHeight: 190
    }, 300)).toEqual({ width: 300, height: 190 });
  });

  it("ignores empty option and hidden-element bounds in the offscreen settings probe", () => {
    const contentRect = {
      left: -10000, top: 0, right: -9730, bottom: 190, width: 270, height: 190
    } as DOMRect;
    const content = {
      querySelectorAll: () => [
        { getBoundingClientRect: () => ({ width: 242, height: 32, right: -9744, bottom: 110 }) },
        { getBoundingClientRect: () => ({ width: 0, height: 0, right: 0, bottom: 0 }) }
      ]
    } as unknown as HTMLElement;

    const extent = measureVisualExtent(content, contentRect);
    expect(extent).toEqual({ visualWidth: 270, visualHeight: 190 });
    expect(resolveWindowSize({
      scrollWidth: 270, scrollHeight: 190,
      boundingWidth: 270, boundingHeight: 190, ...extent
    })).toEqual({ width: MIN_WINDOW_WIDTH, height: MIN_WINDOW_HEIGHT });
  });

  it("still measures visible overflow from an offscreen probe", () => {
    const contentRect = { left: -10000, top: 0, right: -9730, bottom: 190 } as DOMRect;
    const content = {
      querySelectorAll: () => [
        { getBoundingClientRect: () => ({ width: 300, height: 210, right: -9700, bottom: 210 }) }
      ]
    } as unknown as HTMLElement;
    expect(measureVisualExtent(content, contentRect)).toEqual({ visualWidth: 300, visualHeight: 210 });
  });

  it("keeps the measurement-failure fallback when content fits", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: MIN_WINDOW_HEIGHT,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: MIN_WINDOW_HEIGHT
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: MIN_WINDOW_HEIGHT
    });
  });

  it("keeps the compact fallback when content is shorter", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: MIN_WINDOW_HEIGHT - 12,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: MIN_WINDOW_HEIGHT - 12
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: MIN_WINDOW_HEIGHT
    });
  });

  it("uses the measured content height without CSS pixel padding", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: 208,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: 208
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: 208
    });
  });

  it("includes descendants whose painted bounds extend past their container", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: 190,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: 190,
        visualWidth: MIN_WINDOW_WIDTH,
        visualHeight: 218
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: 218
    });
  });

  it("matches content height when platform font metrics exceed the compact baseline", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: MIN_WINDOW_HEIGHT + 0.25,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: MIN_WINDOW_HEIGHT + 0.25
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: MIN_WINDOW_HEIGHT + 1
    });
  });

  it("expands when rendered content exceeds the compact minimum", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: 238.4,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: 238.4
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: 239
    });
  });

  it("falls back to the compact minimum for invalid measurements", () => {
    expect(
      resolveWindowSize({
        scrollWidth: Number.NaN,
        scrollHeight: Number.NaN,
        boundingWidth: Number.NaN,
        boundingHeight: Number.NaN
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: MIN_WINDOW_HEIGHT
    });
  });

  it("clamps oversized content to the maximum window size", () => {
    expect(
      resolveWindowSize({
        scrollWidth: 480,
        scrollHeight: 390,
        boundingWidth: 480,
        boundingHeight: 390
      })
    ).toEqual({
      width: MAX_WINDOW_WIDTH,
      height: MAX_WINDOW_HEIGHT
    });
  });
  it("keeps compact content at the compact width instead of a wider startup viewport", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: MIN_WINDOW_HEIGHT,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: MIN_WINDOW_HEIGHT,
        visualWidth: MIN_WINDOW_WIDTH,
        visualHeight: MIN_WINDOW_HEIGHT
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: MIN_WINDOW_HEIGHT
    });
  });

});
