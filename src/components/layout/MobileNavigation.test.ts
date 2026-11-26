import { describe, expect, it } from "vitest";
import { mobileDestinations } from "./MobileNavigation";

describe("mobile navigation destinations", () => {
  it("uses at most five of Flow's real routes", () => {
    expect(mobileDestinations(true, true).map(item => item.to)).toEqual([
      "/", "/shorts", "/subscriptions", "/music", "/library",
    ]);
  });
  it("respects disabled Shorts and Music navigation", () => {
    expect(mobileDestinations(false, false).map(item => item.to)).toEqual([
      "/", "/subscriptions", "/library",
    ]);
    expect(mobileDestinations(false, true).map(item => item.to)).not.toContain("/shorts");
    expect(mobileDestinations(true, false).map(item => item.to)).not.toContain("/music");
  });
});
