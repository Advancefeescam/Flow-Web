import { describe, expect, it } from "vitest";
import { shortSheetShare, shortSheetTarget } from "./shortSheet";
describe("mobile Shorts sheet", () => {
  it("tracks the drag relative to stage height and clamps it", () => {
    expect(shortSheetShare(.62, 100, 500)).toBeCloseTo(.42);
    expect(shortSheetShare(.62, -1000, 500)).toBe(.85);
    expect(shortSheetShare(.62, 1000, 500)).toBe(.15);
    expect(shortSheetShare(.62, 100, 0)).toBe(.62);
  });
  it("snaps to three sizes or dismisses on a downward pull", () => {
    expect(shortSheetTarget(.42)).toBe(.42);
    expect(shortSheetTarget(.62)).toBe(.62);
    expect(shortSheetTarget(.76)).toBe(.78);
    expect(shortSheetTarget(.2)).toBeNull();
    expect(shortSheetTarget(.42, 1200)).toBeNull();
    expect(shortSheetTarget(.62, -1200)).toBe(.78);
  });
});
