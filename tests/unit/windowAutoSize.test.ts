import { describe, expect, it } from "vitest";
import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
  resolveWindowSize
} from "../../src/features/window/autoSize";

describe("window auto size", () => {
  it("keeps the hidden-startup minimum when content fits", () => {
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

  it("keeps the compact baseline when content is shorter", () => {
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

  it("uses the measured content height instead of a fixed two-line label reserve", () => {
    expect(
      resolveWindowSize({
        scrollWidth: MIN_WINDOW_WIDTH,
        scrollHeight: 208,
        boundingWidth: MIN_WINDOW_WIDTH,
        boundingHeight: 208
      })
    ).toEqual({
      width: MIN_WINDOW_WIDTH,
      height: 212
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
      height: 222
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
      height: MIN_WINDOW_HEIGHT + 5
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
      height: 243
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
});
