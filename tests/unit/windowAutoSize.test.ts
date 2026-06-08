import { describe, expect, it } from "vitest";
import {
  MAX_WINDOW_HEIGHT,
  MAX_WINDOW_WIDTH,
  MIN_WINDOW_HEIGHT,
  MIN_WINDOW_WIDTH,
  resolveWindowSize
} from "../../src/features/window/autoSize";

describe("window auto size", () => {
  it("keeps the compact minimum when content fits", () => {
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
      height: 241
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
