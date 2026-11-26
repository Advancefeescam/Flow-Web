import { afterEach, describe, expect, it, vi } from "vitest";
import { getWebSession, signIn, beginTotp, signOut } from "./webAuth";

afterEach(() => vi.unstubAllGlobals());
describe("Flow Web auth transport", () => {
  it("uses a same-origin cookie session without local token storage", async () => {
    const fetch = vi.fn().mockImplementation(async () => Response.json({ authenticated: true, username: "flow", totpEnabled: false, sessionCount: 1 }));
    vi.stubGlobal("fetch", fetch);
    await expect(getWebSession()).resolves.toMatchObject({ authenticated: true });
    expect(fetch).toHaveBeenCalledWith("/api/v1/auth/session", expect.objectContaining({ method: "GET", credentials: "same-origin", cache: "no-store" }));
    await signIn("flow", "test-password", "123456");
    expect(fetch).toHaveBeenLastCalledWith("/api/v1/auth/login", expect.objectContaining({ method: "POST", body: JSON.stringify({ username: "flow", password: "test-password", code: "123456" }) }));
    await signOut();
    expect(fetch).toHaveBeenLastCalledWith("/api/v1/auth/logout", expect.objectContaining({ method: "POST", body: "{}" }));
  });
  it("reports failed reauthentication rather than pretending enrollment succeeded", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue(Response.json({ message: "Invalid password" }, { status: 401 })));
    await expect(beginTotp("wrong")).rejects.toThrow("Invalid password");
  });
});
