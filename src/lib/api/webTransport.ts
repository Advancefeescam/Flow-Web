import { SESSION_EXPIRED_EVENT } from "./webAuth";

type RpcArgs = Record<string, unknown>;

function resolveMediaUrls(value: unknown, origin: string): unknown {
  if (Array.isArray(value)) return value.map(item => resolveMediaUrls(item, origin));
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [
      key, resolveMediaUrls(item, origin),
    ]));
  }
  return typeof value === "string" && value.startsWith("/media/") && origin
    ? new URL(value, origin).href : value;
}

/** Same-origin transport; browser credentials stay with the instance. */
export async function invokeWeb<T>(
  command: string,
  args: RpcArgs = {},
  request: typeof fetch = fetch,
): Promise<T> {
  if (!/^[a-z][a-z0-9_]{0,79}$/.test(command)) {
    throw { kind: "validation", message: "Invalid backend command" };
  }
  let response: Response;
  try {
    response = await request(`/api/v1/rpc/${command}`, {
      method: "POST",
      credentials: "same-origin",
      cache: "no-store",
      redirect: "error",
      headers: { "Content-Type": "application/json", Accept: "application/json" },
      body: JSON.stringify(args),
      signal: AbortSignal.timeout(90_000),
    });
  } catch {
    throw { kind: "network", message: "Cannot reach the Flow Web server." };
  }
  if (response.status === 401 && typeof window !== "undefined") window.dispatchEvent(new Event(SESSION_EXPIRED_EVENT));
  let data: unknown;
  try {
    data = await response.json();
  } catch {
    throw { kind: response.status === 401 ? "unauthorized" : "network",
      message: `Flow Web returned HTTP ${response.status} instead of JSON.` };
  }
  if (!response.ok) throw data;
  return resolveMediaUrls(data, typeof location === "undefined" ? "" : location.origin) as T;
}
