import { useCallback, useEffect, useState } from "react";
import { CheckCircle2, CircleAlert, LoaderCircle, RefreshCw, Server } from "lucide-react";
import { invokeBackend } from "../../lib/api/errors";
import { Button } from "../ui/Button";

interface Status {
  configured: boolean;
  reachable: boolean;
  pairingAvailable: boolean;
  protocols?: string[];
  live?: boolean;
  error?: string;
}

/** Never present native pairing controls as working before the headless service exists. */
export function WebSyncReadiness() {
  const [status, setStatus] = useState<Status | null>(null);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState("");
  const refresh = useCallback(async () => {
    setBusy(true);
    setError("");
    try { setStatus(await invokeBackend<Status>("web_sync_status")); }
    catch (e) { setError(e instanceof Error ? e.message : "Could not check Flow Sync."); }
    finally { setBusy(false); }
  }, []);
  useEffect(() => { void refresh(); }, [refresh]);
  return <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-4 py-8 sm:px-6">
    <header>
      <h1 className="text-3xl font-bold tracking-tight text-on-surface">Flow Sync</h1>
      <p className="mt-2 text-sm text-on-surface-variant">Connect this Flow Web instance to your existing Flow devices.</p>
    </header>
    <section className="space-y-4 rounded-2xl border border-outline-soft bg-surface-container p-5">
      <div className="flex items-center gap-3">
        <Server size={22} className="text-primary"/>
        <h2 className="flex-1 text-base font-semibold">Sync server</h2>
        <Button size="sm" variant="tonal" disabled={busy} onClick={() => void refresh()}>
          {busy ? <LoaderCircle size={16} className="animate-spin"/> : <RefreshCw size={16}/>} Check
        </Button>
      </div>
      {busy ? <p className="text-sm text-on-surface-variant">Checking connection…</p> :
       error ? <p role="alert" className="text-sm text-error">{error}</p> :
       !status?.configured ? <p className="text-sm text-on-surface-variant">No sync server configured. Set FLOW_WEB_SYNC_URL in your instance configuration and restart.</p> :
       status.reachable ? <div className="space-y-2">
         <p className="flex items-center gap-2 text-sm font-medium"><CheckCircle2 size={18} className="text-primary"/> Server reachable</p>
         <p className="text-sm text-on-surface-variant">Protocols: {status.protocols?.join(", ") || "Not advertised"}</p>
         <p className="text-sm text-on-surface-variant">Live transport: {status.live ? "Available" : "Not advertised"}</p>
       </div> : <p role="alert" className="flex items-center gap-2 text-sm text-error"><CircleAlert size={18} className="shrink-0"/>{status.error || "Server unreachable"}</p>}
    </section>
    <section className="rounded-2xl bg-surface-container-low p-5">
      <h2 className="font-semibold">Device linking is not available yet</h2>
      <p className="mt-2 text-sm leading-relaxed text-on-surface-variant">The server transport check is read-only. This instance has not joined a group and does not send or receive library data. Existing devices and their sync group remain unchanged.</p>
    </section>
  </div>;
}
