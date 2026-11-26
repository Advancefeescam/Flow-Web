import { describe, expect, it } from "vitest";
import { commentSheetBounds, commentSheetSnap } from "./commentSheet";

describe("mobile comments sheet", () => {
  it("starts at the bottom of the visible player", () => {
    expect(commentSheetBounds(915, 288)).toEqual({ top: 288, height: 627, compactOffset: 300.96 });
  });
  it("leaves usable room in short landscape viewports", () => {
    expect(commentSheetBounds(350, 300)).toEqual({ top: 190, height: 160, compactOffset: 0 });
  });
  it("snaps between expanded, compact, and closed using drag position and velocity", () => {
    expect(commentSheetSnap(600, 25)).toBe("expanded");
    expect(commentSheetSnap(600, 290)).toBe("compact");
    expect(commentSheetSnap(600, 550)).toBe("closed");
    expect(commentSheetSnap(600, 350, 1200)).toBe("closed");
    expect(commentSheetSnap(600, 190, -1000)).toBe("expanded");
  });
});
