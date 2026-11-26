export interface WebSession {
  authenticated: boolean;
  username: string;
  totpEnabled: boolean;
  sessionCount: number;
}
export interface Enrollment { secret: string; uri: string }
export const SESSION_EXPIRED_EVENT = "flow-web-session-expired";

async function authRequest<T>(path: string, args?: Record<string, string>): Promise<T> {
  const response = await fetch("/api/v1/auth/" + path, {
    method: args ? "POST" : "GET",
    credentials: "same-origin",
    cache: "no-store",
    redirect: "error",
    headers: args ? { "Content-Type": "application/json", Accept: "application/json" } : { Accept: "application/json" },
    body: args ? JSON.stringify(args) : undefined,
    signal: AbortSignal.timeout(15000),
  });
  const result = await response.json().catch(() => null);
  if (!response.ok || !result) throw new Error(result?.message || "Flow Web authentication is unavailable.");
  return result as T;
}
export const getWebSession = () => authRequest<WebSession>("session");
export const signIn = (username: string, password: string, code: string) =>
  authRequest<WebSession>("login", { username, password, code });
export const signOut = () => authRequest<WebSession>("logout", {});
export const beginTotp = (password: string) => authRequest<Enrollment>("totp/begin", { password });
export const confirmTotp = (password: string, code: string) =>
  authRequest<WebSession>("totp/confirm", { password, code });
export const disableTotp = (password: string, code: string) =>
  authRequest<WebSession>("totp/disable", { password, code });
export const cancelTotp = () => authRequest<WebSession>("totp/cancel", {});
export const revokeOtherSessions = (password: string, code: string) =>
  authRequest<WebSession>("revoke", { password, code });
