import { useEffect, useState, type FormEvent, type ReactNode, type CSSProperties } from "react";
import { ShieldCheck, LoaderCircle, WifiOff } from "lucide-react";
import { motion, useReducedMotion } from "framer-motion";
import { AuthUsernameField, AuthPasswordField, AuthCodeField, AuthError } from "./AuthFields";
import { DEFAULT_THEME, scopeVariables } from "../../lib/themes";
import { useSystemDarkMode } from "../../lib/useSystemDarkMode";
import { Button } from "../ui/Button";
import { Logo } from "../common/Logo";
import { getWebSession, signIn, SESSION_EXPIRED_EVENT, type WebSession } from "../../lib/api/webAuth";

function AuthSurface({ children }: { children: ReactNode }) {
  const systemDark = useSystemDarkMode();
  const reducedMotion = useReducedMotion();
  // boot-theme.js restores the last selected Flow palette before React starts.
  // A fresh browser uses the normal Flow system-mode palette.
  const snapshot = document.documentElement.dataset.themeVariant;
  const variant = systemDark ? "dark" : "light";
  const style = snapshot ? undefined : { ...scopeVariables(DEFAULT_THEME, variant), colorScheme: variant } as CSSProperties;
  return <main style={style} className="h-dvh w-full overflow-y-auto bg-background text-on-surface">
    <div className="min-h-full flex flex-col items-center justify-center px-4 py-8 sm:px-8">
      <motion.section initial={reducedMotion ? false : { opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.2 }} className="w-full max-w-[420px] rounded-[28px] border border-outline-soft bg-surface-container px-6 py-7 sm:px-9 sm:py-9">
        {children}
      </motion.section>
      <p className="mt-5 flex items-center justify-center gap-2 text-xs text-chrome-neutral-400"><ShieldCheck size={14}/> Your private Flow Web instance</p>
    </div>
  </main>;
}

function Login({ session, onSuccess }: { session: WebSession; onSuccess: (session: WebSession) => void }) {
  const [username, setUsername] = useState(session.username);
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (busy) return;
    setBusy(true); setError("");
    try { onSuccess(await signIn(username, password, code)); setPassword(""); setCode(""); }
    catch (e) {
      setError(e instanceof Error ? e.message : "Could not sign in.");
      setCode("");
      // Enrollment may have changed from another browser.
      try { const latest = await getWebSession(); if (latest.totpEnabled !== session.totpEnabled) onSuccess(latest); } catch { /* keep form */ }
    } finally { setBusy(false); }
  }
  return <AuthSurface>
    <header className="flex flex-col items-center text-center mb-7">
      <Logo size={64} className="mb-4 [&_svg]:!cursor-default [&_svg]:!transform-none"/>
      <h1 className="text-[28px] font-bold leading-tight tracking-tight text-chrome-neutral-100">Sign in to Flow</h1>
      <p className="mt-2 text-sm leading-relaxed text-chrome-neutral-400">Sign in to your self-hosted library.</p>
    </header>
    <form className="space-y-5" onSubmit={submit}>
      <AuthUsernameField value={username} onChange={setUsername} disabled={busy}/>
      <AuthPasswordField value={password} onChange={setPassword} disabled={busy} autoFocus/>
      {session.totpEnabled && <AuthCodeField value={code} onChange={setCode} disabled={busy}/>}
      <AuthError message={error}/>
      <Button type="submit" size="lg" className="w-full focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-surface-container"
        disabled={busy || !password || !username || (session.totpEnabled && code.length !== 6)}>
        {busy && <LoaderCircle size={18} className="animate-spin"/>}{busy ? "Signing in…" : "Sign in"}
      </Button>
    </form>
  </AuthSurface>;
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
  return <AuthSurface>
    <div className="flex flex-col items-center gap-4 text-center py-4">
      <Logo size={64} className="[&_svg]:!cursor-default [&_svg]:!transform-none"/>
      {failed ? <>
        <WifiOff size={24} className="text-chrome-neutral-400"/>
        <h1 className="text-lg font-semibold">Unable to connect</h1>
        <p role="alert" className="text-sm text-chrome-neutral-400">Could not connect to your Flow Web server.</p>
        <Button onClick={() => setRetry(n => n + 1)}>Try again</Button>
      </> : <LoaderCircle className="animate-spin text-primary" aria-label="Checking session"/>}
    </div>
  </AuthSurface>;
}
