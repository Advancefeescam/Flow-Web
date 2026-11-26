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
  return <main style={style} className="flow-web-auth-surface h-dvh w-full overflow-y-auto bg-background text-on-surface">
    <div className="mx-auto grid min-h-full w-full max-w-[1440px] lg:grid-cols-[minmax(0,1.1fr)_minmax(0,1fr)]">
      <aside className="relative m-5 hidden min-h-[480px] flex-col justify-between overflow-hidden rounded-[32px] bg-surface-container p-10 lg:flex xl:m-8 xl:p-14">
        <div className="relative z-10 flex items-center gap-2">
          <Logo size={42} className="[&_svg]:!cursor-default [&_svg]:!transform-none"/>
          <span className="text-xl font-bold tracking-tight">Flow Web</span>
        </div>
        <div aria-hidden="true" className="pointer-events-none absolute inset-0 grid place-items-center overflow-hidden">
          <div className="absolute h-[76%] w-[76%] max-h-[450px] max-w-[450px] rotate-[-12deg] rounded-[26%] border border-primary/15 bg-surface-container-high/50"/>
          <div className="absolute h-[55%] w-[55%] max-h-[320px] max-w-[320px] rotate-[10deg] rounded-[28%] border border-primary/20 bg-surface-container-highest/60"/>
          <Logo size={160} className="relative [&_svg]:!cursor-default [&_svg]:!transform-none"/>
        </div>
        <div className="relative z-10">
          <h2 className="max-w-[360px] text-4xl font-semibold leading-[1.08] tracking-tight xl:text-5xl">Your library, on your terms.</h2>
          <p className="mt-4 max-w-[320px] text-sm leading-relaxed text-on-surface-variant">Video, music and the things you save. All in your own Flow instance.</p>
        </div>
      </aside>
      <div className="flow-auth-center flex min-w-0 flex-col items-center justify-center px-6 py-10 sm:px-10 lg:px-14">
        <motion.section initial={reducedMotion ? false : { opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.22 }} className="flow-auth-card w-full max-w-[400px]">
          <div className="mb-10 flex items-center gap-2 lg:hidden">
            <Logo size={42} className="[&_svg]:!cursor-default [&_svg]:!transform-none"/>
            <span className="text-xl font-bold tracking-tight">Flow Web</span>
          </div>
          {children}
          <div className="mt-8 flex items-center gap-2 border-t border-outline-soft pt-5 text-xs text-on-surface-variant">
            <ShieldCheck size={16} className="shrink-0"/> Private self-hosted instance
          </div>
          {window.location.protocol !== "https:" && <p className="mt-2 text-xs leading-relaxed text-on-surface-variant">Local HTTP connection. Use HTTPS for remote access.</p>}
        </motion.section>
      </div>
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
    <header className="mb-8">
      <h1 className="text-[34px] font-semibold leading-tight tracking-tight text-on-surface sm:text-[40px]">Sign in</h1>
      <p className="mt-2 text-sm leading-relaxed text-on-surface-variant">Welcome back to your Flow library.</p>
    </header>
    <form className="space-y-5" onSubmit={submit}>
      <AuthUsernameField value={username} onChange={setUsername} disabled={busy}/>
      <AuthPasswordField value={password} onChange={setPassword} disabled={busy} autoFocus={!window.matchMedia?.("(pointer: coarse)")?.matches}/>
      {session.totpEnabled && <AuthCodeField value={code} onChange={setCode} disabled={busy}/>}
      <AuthError message={error}/>
      <Button type="submit" size="lg" className="w-full focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 focus-visible:ring-offset-background"
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
