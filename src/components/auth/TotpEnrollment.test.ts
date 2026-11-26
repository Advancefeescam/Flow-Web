import { act, createElement } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { TotpEnrollment } from "./TotpEnrollment";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

afterEach(() => vi.restoreAllMocks());

describe("Flow Web authenticator enrollment", () => {
  it("renders a local QR and reveals the manual key only when requested", () => {
    const host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    const secret = "JBSWY3DPEHPK3PXP";
    const uri = "otpauth://totp/Flow%20Web:flow?secret=" + secret + "&issuer=Flow%20Web";
    try {
      act(() => root.render(createElement(TotpEnrollment, { enrollment: { secret, uri } })));
      expect(host.querySelector("svg[viewBox]")).not.toBeNull();
      expect(host.querySelector("img")).toBeNull();
      expect(host.querySelector("code")).toBeNull();
      const toggle = host.querySelector("button[aria-expanded]") as HTMLButtonElement;
      expect(toggle.getAttribute("aria-expanded")).toBe("false");
      act(() => toggle.click());
      expect(toggle.getAttribute("aria-expanded")).toBe("true");
      expect(host.querySelector("code")?.textContent?.replace(/\s/g, "")).toBe(secret);
      expect(host.querySelector("a")?.getAttribute("href")).toBe(uri);
      act(() => toggle.click());
      expect(host.querySelector("code")).toBeNull();
    } finally { act(() => root.unmount()); host.remove(); }
  });
});
