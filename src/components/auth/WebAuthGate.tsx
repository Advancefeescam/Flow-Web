import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { Eye, EyeOff, KeyRound, ShieldCheck, LoaderCircle } from "lucide-react";
import { Button } from "../ui/Button";
import { getWebSession, signIn, SESSION_EXPIRED_EVENT, type WebSession } from "../../lib/api/webAuth";

export const authInputClass = "w-full rounded-2xl border border-outline-variant bg-surface-container-low px-4 py-3.5 text-on-surface placeholder:text-on-surface-variant focus-visible:outline-2 focus-visible:outline-primary disabled:opacity-50";

function Login({ session, onSuccess }: { session: WebSession; onSuccess: (session: WebSession) => void }) {
  const [username, setUsername] = useState(session.username);
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [visible, setVisible] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    setBusy(true); setError("");
    try { onSuccess(await signIn(username, password, code)); setPassword(""); setCode(""); }
    catch (e) {
      setError(e instanceof Error ? e.message : "Could not sign in.");
      // Enrollment may have changed from another browser.
      try { const latest = await getWebSession(); if (latest.totpEnabled !== session.totpEnabled) onSuccess(latest); } catch { /* keep form */ }
    } finally { setBusy(false); }
  }
  return (
    <main className="min-h-dvh w-full flex items-center justify-center bg-background text-on-surface p-5 sm:p-8 overflow-y-auto">
      <section className="w-full max-w-[440px] bg-surface-container rounded-[32px] p-6 sm:p-9 space-y-7" aria-labelledby="web-login-title">
        <header className="flex flex-col items-center text-center gap-3">
          <img src="/flow-web-logo.svg" alt="" className="w-[72px] h-[72px]" />
          <div>
            <h1 id="web-login-title" className="text-[30px] leading-tight font-bold">Sign in to Flow</h1>
            <p className="text-sm text-on-surface-variant mt-2">Your self-hosted YouTube and YouTube Music library.</p>
          </div>
        </header>
        <form className="space-y-4" onSubmit={submit}>
          <label className="block space-y-1.5 text-sm font-medium">
            <span className="px-1">Username</span>
            <input className={authInputClass} autoComplete="username" value={username} onChange={e => setUsername(e.target.value)} required disabled={busy} />
          </label>
          <label className="block space-y-1.5 text-sm font-medium">
            <span className="px-1">Password</span>
            <div className="relative">
              <input className={authInputClass + " pe-14"} type={visible ? "text" : "password"} autoComplete="current-password" value={password} onChange={e => setPassword(e.target.value)} required disabled={busy} />
              <button className="absolute end-2 top-1/2 -translate-y-1/2 w-10 h-10 rounded-full flex items-center justify-center text-on-surface-variant hover:bg-surface-container-high" type="button" onClick={() => setVisible(v => !v)} aria-label={visible ? "Hide password" : "Show password"}>{visible ? <EyeOff size={20} /> : <Eye size={20} />}</button>
            </div>
          </label>
          {session.totpEnabled && <label className="block space-y-1.5 text-sm font-medium">
            <span className="px-1 flex items-center gap-2"><KeyRound size={16}/> Verification code</span>
            <input className={authInputClass + " tracking-[0.3em] font-mono"} value={code} onChange={e => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))} inputMode="numeric" autoComplete="one-time-code" pattern="[0-9]{6}" required disabled={busy} placeholder="000000" />
            <span className="block text-xs text-on-surface-variant font-normal">Enter the current six-digit code from your authenticator app.</span>
          </label>}
          {error && <p role="alert" className="text-sm text-error rounded-xl bg-surface-container-high px-3 py-2">{error}</p>}
          <Button type="submit" size="lg" className="w-full !h-14" disabled={busy || !password || !username || (session.totpEnabled && code.length !== 6)}>
            {busy && <LoaderCircle size={18} className="animate-spin"/>}{busy ? "Signing in…" : "Sign in"}
          </Button>
        </form>
        <footer className="flex items-center justify-center gap-2 text-xs text-on-surface-variant"><ShieldCheck size={16}/> This is your private Flow Web instance.</footer>
      </section>
    </main>
  );
}

export function WebAuthGate({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<WebSession | null>(null);
  const [failed, setFailed] = useState(false);
  const [retry, setRetry] = useState(0);
  useEffect(() => {
    let alive = true;
    setFailed(false);
    getWebSession().then(result => { if (alive) setSession(result); }).catch(() => { if (alive) setFailed(true); });
    return () => { alive = false; };
  }, [retry]);
  useEffect(() => {
    if (!session?.authenticated) return;
    const verify = () => { void getWebSession().then(s => { if (!s.authenticated) window.location.reload(); }).catch(() => {}); };
    const expired = () => verify();
    const visible = () => { if (document.visibilityState === "visible") verify(); };
    const timer = window.setInterval(verify, 60000);
    window.addEventListener(SESSION_EXPIRED_EVENT, expired);
    document.addEventListener("visibilitychange", visible);
    return () => { window.clearInterval(timer); window.removeEventListener(SESSION_EXPIRED_EVENT, expired); document.removeEventListener("visibilitychange", visible); };
  }, [session?.authenticated]);
  if (session?.authenticated) return <>{children}</>;
  if (session) return <Login session={session} onSuccess={setSession}/>;
  return <main className="min-h-dvh flex flex-col gap-4 items-center justify-center bg-background text-on-surface p-6">
    <img src="/flow-web-logo.svg" className="w-16 h-16" alt="Flow Web"/>
    {failed ? <><p role="alert">Could not connect to your Flow Web server.</p><Button onClick={() => setRetry(n => n + 1)}>Try again</Button></> : <LoaderCircle className="animate-spin text-primary" aria-label="Checking session"/>}
  </main>;
}
