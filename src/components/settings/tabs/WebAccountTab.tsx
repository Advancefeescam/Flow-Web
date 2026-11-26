import { useEffect, useState, type FormEvent } from "react";
import { ShieldCheck, KeyRound, LogOut, Monitor, Copy, UserRound } from "lucide-react";
import { Button } from "../../ui/Button";
import { SettingsGroup } from "../SettingsGroup";
import { SettingItem } from "../SettingItem";
import { authInputClass } from "../../auth/WebAuthGate";
import { getWebSession, signOut, beginTotp, confirmTotp, disableTotp, cancelTotp, revokeOtherSessions, type WebSession, type Enrollment } from "../../../lib/api/webAuth";
import { getString } from "../../../lib/i18n/index";

type Action = "setup" | "disable" | "revoke" | null;
export function WebAccountTab() {
  const [session, setSession] = useState<WebSession | null>(null);
  const [action, setAction] = useState<Action>(null);
  const [enrollment, setEnrollment] = useState<Enrollment | null>(null);
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  useEffect(() => {
    let alive = true;
    getWebSession().then(s => { if (alive) setSession(s); }).catch(e => { if (alive) setError(e.message); });
    return () => { alive = false; };
  }, []);
  const clear = () => { setPassword(""); setCode(""); setEnrollment(null); setAction(null); };
  const open = (next: Action) => { clear(); setError(""); setNotice(""); setAction(next); };
  async function run(event: FormEvent) {
    event.preventDefault();
    if (busy || !action || !session) return;
    setBusy(true); setError(""); setNotice("");
    try {
      if (action === "setup" && !enrollment) {
        setEnrollment(await beginTotp(password));
        setPassword("");
        return;
      }
      const updated = action === "setup" ? await confirmTotp(password, code)
        : action === "disable" ? await disableTotp(password, code)
        : await revokeOtherSessions(password, code);
      setSession(updated);
      setNotice(action === "setup" ? "Two-step verification is enabled. Other sessions were signed out."
        : action === "disable" ? "Two-step verification is disabled." : "All other sessions were signed out.");
      clear();
    } catch (e) { setError(e instanceof Error ? e.message : "Security settings could not be updated."); }
    finally { setBusy(false); }
  }
  async function cancel() {
    if (busy) return;
    setBusy(true);
    try { if (enrollment) await cancelTotp(); clear(); setError(""); }
    catch (e) { setError(e instanceof Error ? e.message : "Could not cancel setup."); }
    finally { setBusy(false); }
  }
  async function logout() {
    if (busy) return;
    setBusy(true);
    try { await signOut(); window.location.reload(); }
    catch (e) { setError(e instanceof Error ? e.message : "Could not sign out."); setBusy(false); }
  }
  const needsCode = Boolean(enrollment || (session?.totpEnabled && action !== "setup"));
  return <div className="space-y-6 pb-8 max-w-3xl">
    <div className="space-y-1">
      <h2 className="text-2xl font-bold text-on-surface">{getString("settings_web_account")}</h2>
      <p className="text-sm text-on-surface-variant">Manage access to this Flow Web instance.</p>
    </div>
    <SettingsGroup title="Your account">
      <SettingItem title="Administrator" description={session?.username ?? "Loading…"} icon={<UserRound size={20}/>}/>
      <SettingItem title={getString("settings_web_sign_out")} description="End this browser session." icon={<LogOut size={20}/>} disabled={!session || busy} onSelect={() => void logout()}/>
    </SettingsGroup>
    <SettingsGroup title="Security">
      <SettingItem title={getString("settings_web_two_factor")} description={session ? (session.totpEnabled ? "Enabled · Authenticator app" : "Disabled · Add an authenticator app") : "Loading…"} icon={<ShieldCheck size={20}/>} disabled={!session || busy} onSelect={() => open(session?.totpEnabled ? "disable" : "setup")}/>
      <SettingItem title={getString("settings_web_sessions")} description={session ? session.sessionCount + " active session(s), including this browser" : "Loading…"} icon={<Monitor size={20}/>} disabled={!session || busy} onSelect={() => open("revoke")}/>
    </SettingsGroup>
    {action && <section className="rounded-3xl bg-surface-container p-5 space-y-4" aria-labelledby="security-action">
      <h3 id="security-action" className="font-semibold text-on-surface flex items-center gap-2"><KeyRound size={20}/>{action === "setup" ? "Set up two-step verification" : action === "disable" ? "Disable two-step verification" : "Sign out other sessions"}</h3>
      {enrollment && <div className="space-y-3">
        <p className="text-sm text-on-surface-variant">Add a time-based account to your authenticator app. Use this setup key, six digits and a 30-second period. Setup expires in five minutes.</p>
        <div className="rounded-xl bg-surface-container-high p-3 break-all font-mono text-sm select-all text-on-surface">{enrollment.secret}</div>
        <div className="flex flex-wrap gap-2">
          <Button variant="tonal" type="button" onClick={() => { if (!navigator.clipboard) { setError("Select and copy the setup key manually."); return; } void navigator.clipboard.writeText(enrollment.secret).then(() => setNotice("Setup key copied.")).catch(() => setError("Select and copy the setup key manually.")); }}><Copy size={16}/> Copy key</Button>
          <a className="inline-flex items-center rounded-full px-4 py-2 text-primary text-sm font-medium" href={enrollment.uri}>Open authenticator</a>
        </div>
        <p className="text-xs text-on-surface-variant">Keep the setup key private. If you lose your authenticator, server-owner recovery is required.</p>
      </div>}
      <form className="space-y-3" onSubmit={run}>
        <label className="block space-y-1 text-sm text-on-surface"><span>Administrator password</span>
          <input type="password" autoComplete="current-password" className={authInputClass} required value={password} onChange={e => setPassword(e.target.value)} disabled={busy}/>
        </label>
        {needsCode && <label className="block space-y-1 text-sm text-on-surface"><span>Verification code</span>
          <input className={authInputClass + " font-mono tracking-widest"} inputMode="numeric" autoComplete="one-time-code" pattern="[0-9]{6}" required value={code} onChange={e => setCode(e.target.value.replace(/\D/g, "").slice(0, 6))} disabled={busy}/>
        </label>}
        <div className="flex justify-end flex-wrap gap-2">
          <Button type="button" variant="ghost" onClick={() => void cancel()} disabled={busy}>Cancel</Button>
          <Button type="submit" disabled={busy || !password || (needsCode && code.length !== 6)}>{busy ? "Please wait…" : action === "setup" && !enrollment ? "Continue" : action === "setup" ? "Verify and enable" : action === "disable" ? "Disable" : "Sign out other sessions"}</Button>
        </div>
      </form>
    </section>}
    {error && <p role="alert" className="text-sm text-error">{error}</p>}
    {notice && <p role="status" className="text-sm text-on-surface-variant">{notice}</p>}
    <p className="text-xs text-on-surface-variant">The server owner manages the administrator password through FLOW_WEB_PASSWORD. This version has one administrator account; separate user accounts and passkeys are not yet available.</p>
  </div>;
}
