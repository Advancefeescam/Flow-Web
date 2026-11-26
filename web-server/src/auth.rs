//! Single-instance administrator authentication. No Basic-auth bypass.
use axum::{
    Json, body::to_bytes,
    extract::{MatchedPath, Request, State},
    http::{HeaderMap, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, fs, io::Write, path::{Path, PathBuf}, sync::{Arc, Mutex}, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};
use subtle::ConstantTimeEq;
use uuid::Uuid;

const SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);
const ENROLL_TTL: Duration = Duration::from_secs(5 * 60);
const ATTEMPT_WINDOW: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: u32 = 8;
const MAX_SESSIONS: usize = 64;
const COOKIE: &str = "flow_web_session";

#[derive(Clone)]
pub struct Config {
    digest: [u8; 32],
    origin: String,
    secure: bool,
    path: PathBuf,
    state: Arc<Mutex<AuthState>>,
}
#[derive(Clone, Serialize, Deserialize, Default)]
struct Persisted {
    secret: Option<Vec<u8>>,
    last_step: Option<u64>,
}
struct Session {
    expires: Instant,
    pending: Option<(Vec<u8>, Instant)>,
}
struct AuthState {
    healthy: bool,
    persisted: Persisted,
    sessions: HashMap<[u8; 32], Session>,
    attempts: u32,
    window: Instant,
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    #[serde(default)]
    username: String,
    #[serde(default)]
    password: String,
    #[serde(default)]
    code: String,
}
fn digest(value: &str) -> [u8; 32] { Sha256::digest(value.as_bytes()).into() }
fn now_step() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() / 30
}
fn random_secret() -> Vec<u8> {
    [Uuid::new_v4().as_bytes().as_slice(), Uuid::new_v4().as_bytes().as_slice()].concat()
}
fn base32(bytes: &[u8]) -> String {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut result = String::new();
    let mut bits = 0u32;
    let mut count = 0u32;
    for &byte in bytes {
        bits = (bits << 8) | u32::from(byte);
        count += 8;
        while count >= 5 {
            count -= 5;
            result.push(alphabet[((bits >> count) & 31) as usize] as char);
        }
    }
    if count > 0 { result.push(alphabet[((bits << (5 - count)) & 31) as usize] as char); }
    result
}
fn totp(secret: &[u8], step: u64) -> String {
    let mut mac = <Hmac<Sha1> as KeyInit>::new_from_slice(secret).expect("HMAC accepts any key length");
    mac.update(&step.to_be_bytes());
    let bytes = mac.finalize().into_bytes();
    let offset = usize::from(bytes[19] & 15);
    let value = u32::from_be_bytes(bytes[offset..offset+4].try_into().unwrap()) & 0x7fff_ffff;
    format!("{:06}", value % 1_000_000)
}
fn verify_code(secret: &[u8], code: &str, step: u64, previous: Option<u64>) -> Option<u64> {
    if code.len() != 6 || !code.bytes().all(|c| c.is_ascii_digit()) { return None; }
    [step.saturating_sub(1), step, step.saturating_add(1)].into_iter().find(|&candidate| {
        previous.is_none_or(|last| candidate > last) &&
            bool::from(totp(secret, candidate).as_bytes().ct_eq(code.as_bytes()))
    })
}
fn persist(path: &Path, state: &Persisted) -> std::io::Result<()> {
    let tmp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)] {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&tmp)?;
        serde_json::to_writer(&mut file, state).map_err(std::io::Error::other)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&tmp, path)?;
        #[cfg(unix)]
        if let Some(parent) = path.parent() { fs::File::open(parent)?.sync_all()?; }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_file(tmp); }
    result
}

impl Config {
    pub fn from_env(data: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let password = std::env::var("FLOW_WEB_PASSWORD")?;
        if password.len() < 16 { return Err("FLOW_WEB_PASSWORD must have at least 16 bytes".into()); }
        let raw = std::env::var("FLOW_WEB_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8780".into());
        let url = reqwest::Url::parse(&raw)?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
            || url.path() != "/" || url.query().is_some() || url.fragment().is_some()
            || !url.username().is_empty() || url.password().is_some() {
            return Err("FLOW_WEB_ORIGIN must be an HTTP(S) origin without a path".into());
        }
        let path = data.join("web-auth.json");
        let persisted = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Persisted>(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Persisted::default(),
            Err(e) => return Err(e.into()),
        };
        if persisted.secret.as_ref().is_some_and(|s| s.len() != 32)
            || (persisted.secret.is_none() && persisted.last_step.is_some()) {
            return Err("Invalid web authentication state; repair it locally before starting".into());
        }
        let secure = url.scheme() == "https";
        if !secure {
            tracing::warn!("Flow Web is configured for HTTP. Use only a trusted local connection; public access requires HTTPS.");
        }
        Ok(Self {
            digest: digest(&password), origin: url.origin().ascii_serialization(), secure, path,
            state: Arc::new(Mutex::new(AuthState { healthy: true, persisted, sessions: HashMap::new(), attempts: 0, window: Instant::now() })),
        })
    }
    fn valid_password(&self, password: &str) -> bool {
        bool::from(self.digest.ct_eq(&digest(password)))
    }
    fn cookie(&self, value: &str, max_age: u64) -> String {
        format!("{COOKIE}={value}; Path=/; HttpOnly; SameSite=Strict; Max-Age={max_age}{}",
            if self.secure { "; Secure" } else { "" })
    }
    fn save(&self, state: &mut AuthState, value: Persisted) -> Result<(), Response> {
        if let Err(e) = persist(&self.path, &value) {
            tracing::error!("Could not persist authentication state: {e}");
            // Fail closed after an ambiguous disk commit; a restart reloads the durable state.
            state.sessions.clear();
            state.healthy = false;
            return Err(error(StatusCode::INTERNAL_SERVER_ERROR, "Could not save security settings. Check the server."));
        }
        state.persisted = value;
        Ok(())
    }
}
fn session_key(headers: &HeaderMap) -> Option<[u8; 32]> {
    let mut values = headers.get_all(header::COOKIE).iter().filter_map(|h| h.to_str().ok())
        .flat_map(|h| h.split(';')).filter_map(|part| part.trim().strip_prefix("flow_web_session="));
    let token = values.next()?;
    if values.next().is_some() || token.len() != 64 || !token.bytes().all(|c| c.is_ascii_hexdigit()) { return None; }
    Some(digest(token))
}
fn error(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({"kind": if status == StatusCode::UNAUTHORIZED { "unauthorized" } else { "auth" }, "message": message}))).into_response()
}
fn ok(value: Value) -> Response { Json(value).into_response() }
fn status(state: &AuthState, key: Option<[u8; 32]>) -> Value {
    let authenticated = key.is_some_and(|k| state.sessions.contains_key(&k));
    json!({"authenticated": authenticated, "username": "flow", "totpEnabled": state.persisted.secret.is_some(),
        "sessionCount": if authenticated { state.sessions.len() } else { 0 }})
}
fn attempt(state: &mut AuthState) -> Result<(), Response> {
    if state.window.elapsed() >= ATTEMPT_WINDOW { state.window = Instant::now(); state.attempts = 0; }
    if state.attempts >= MAX_ATTEMPTS {
        return Err((StatusCode::TOO_MANY_REQUESTS, [(header::RETRY_AFTER, "60")],
            Json(json!({"kind":"auth","message":"Too many attempts. Try again in a minute."}))).into_response());
    }
    state.attempts += 1;
    Ok(())
}
fn invalid() -> Response { error(StatusCode::UNAUTHORIZED, "Invalid username, password or verification code.") }
fn dispatch(config: &Config, path: &str, method: &Method, key: Option<[u8; 32]>, input: Input) -> Result<Response, Response> {
    let mut state = config.state.lock().map_err(|_| error(StatusCode::INTERNAL_SERVER_ERROR, "Authentication unavailable."))?;
    if !state.healthy { return Err(error(StatusCode::SERVICE_UNAVAILABLE, "Authentication storage unavailable. Restart after repairing it.")); }
    state.sessions.retain(|_, s| s.expires > Instant::now());
    if path == "/api/v1/auth/session" && *method == Method::GET {
        return Ok(ok(status(&state, key)));
    }
    if *method != Method::POST { return Err(error(StatusCode::METHOD_NOT_ALLOWED, "Method not allowed.")); }
    if path == "/api/v1/auth/login" {
        attempt(&mut state)?;
        if input.username != "flow" || !config.valid_password(&input.password) { return Err(invalid()); }
        if let Some(secret) = state.persisted.secret.as_ref() {
            let Some(step) = verify_code(secret, &input.code, now_step(), state.persisted.last_step) else { return Err(invalid()); };
            let mut next = state.persisted.clone();
            next.last_step = Some(step);
            config.save(&mut state, next)?;
        }
        if let Some(key) = key { state.sessions.remove(&key); }
        if state.sessions.len() >= MAX_SESSIONS { return Err(error(StatusCode::TOO_MANY_REQUESTS, "Too many active sessions. Sign out another device or restart the server.")); }
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        let key = digest(&token);
        state.sessions.insert(key, Session { expires: Instant::now() + SESSION_TTL, pending: None });
        state.attempts = 0;
        let mut response = ok(status(&state, Some(key)));
        response.headers_mut().insert(header::SET_COOKIE, config.cookie(&token, SESSION_TTL.as_secs()).parse().unwrap());
        return Ok(response);
    }
    let Some(key) = key.filter(|k| state.sessions.contains_key(k)) else {
        return Err(error(StatusCode::UNAUTHORIZED, "Sign in to Flow Web."));
    };
    match path {
        "/api/v1/auth/logout" => {
            state.sessions.remove(&key);
            let mut response = ok(status(&state, None));
            response.headers_mut().insert(header::SET_COOKIE, config.cookie("", 0).parse().unwrap());
            Ok(response)
        }
        "/api/v1/auth/totp/cancel" => {
            state.sessions.get_mut(&key).unwrap().pending = None;
            Ok(ok(status(&state, Some(key))))
        }
        "/api/v1/auth/totp/begin" | "/api/v1/auth/totp/confirm" | "/api/v1/auth/totp/disable" | "/api/v1/auth/revoke" => {
            attempt(&mut state)?;
            if !config.valid_password(&input.password) { return Err(invalid()); }
            match path {
                "/api/v1/auth/totp/begin" => {
                    if state.persisted.secret.is_some() { return Err(error(StatusCode::CONFLICT, "Two-step verification is already enabled.")); }
                    let secret = random_secret();
                    let encoded = base32(&secret);
                    state.sessions.get_mut(&key).unwrap().pending = Some((secret, Instant::now() + ENROLL_TTL));
                    Ok(ok(json!({"secret":encoded, "uri":format!("otpauth://totp/Flow%20Web:flow?secret={encoded}&issuer=Flow%20Web&algorithm=SHA1&digits=6&period=30")})))
                }
                "/api/v1/auth/totp/confirm" => {
                    if state.persisted.secret.is_some() { return Err(error(StatusCode::CONFLICT, "Two-step verification is already enabled.")); }
                    let Some((secret, _)) = state.sessions.get(&key).and_then(|s| s.pending.clone()).filter(|(_, expiry)| *expiry > Instant::now()) else {
                        return Err(error(StatusCode::BAD_REQUEST, "Setup expired. Start again."));
                    };
                    let Some(step) = verify_code(&secret, &input.code, now_step(), None) else { return Err(invalid()); };
                    config.save(&mut state, Persisted { secret: Some(secret), last_step: Some(step) })?;
                    state.sessions.retain(|k, _| *k == key);
                    state.sessions.get_mut(&key).unwrap().pending = None;
                    state.attempts = 0;
                    Ok(ok(status(&state, Some(key))))
                }
                _ => {
                    if let Some(secret) = state.persisted.secret.as_ref() {
                        let Some(step) = verify_code(secret, &input.code, now_step(), state.persisted.last_step) else { return Err(invalid()); };
                        let mut next = state.persisted.clone();
                        next.last_step = Some(step);
                        if path.ends_with("/disable") { next = Persisted::default(); }
                        config.save(&mut state, next)?;
                    }
                    state.sessions.retain(|k, _| *k == key);
                    state.attempts = 0;
                    Ok(ok(status(&state, Some(key))))
                }
            }
        }
        _ => Err(error(StatusCode::NOT_FOUND, "Unknown authentication endpoint.")),
    }
}

pub async fn guard(State(config): State<Config>, request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let headers = request.headers();
    let key = session_key(headers);
    let read = method == Method::GET || method == Method::HEAD;
    let matched = request.extensions().get::<MatchedPath>().map(|p| p.as_str()).unwrap_or("");
    let protected = [&path[..], matched].iter().any(|p| *p == "/api" || *p == "/media" || p.starts_with("/api/") || p.starts_with("/media/"));
    let cross_origin = (protected && headers.get("sec-fetch-site").and_then(|v| v.to_str().ok()) == Some("cross-site"))
        || (!read && headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) != Some(config.origin.as_str()));
    let mut response = if cross_origin {
        error(StatusCode::FORBIDDEN, "Cross-origin request refused.")
    } else if path.starts_with("/api/v1/auth/") {
        let input = if method == Method::POST {
            match to_bytes(request.into_body(), 8192).await {
                Ok(bytes) => serde_json::from_slice::<Input>(&bytes).ok(),
                Err(_) => None,
            }
        } else { Some(Input::default()) };
        match input {
            Some(input) => dispatch(&config, &path, &method, key, input).unwrap_or_else(|response| response),
            None => error(StatusCode::BAD_REQUEST, "Invalid authentication request."),
        }
    } else {
        let authorized = config.state.lock().ok().is_some_and(|mut state| {
            state.sessions.retain(|_, s| s.expires > Instant::now());
            state.healthy && key.is_some_and(|k| state.sessions.contains_key(&k))
        });
        if (protected || !read) && !authorized {
            error(StatusCode::UNAUTHORIZED, "Sign in to Flow Web.")
        } else { next.run(request).await }
    };
    response.headers_mut().insert(header::CACHE_CONTROL, "private, no-store".parse().unwrap());
    response.headers_mut().insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    response.headers_mut().insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());
    response.headers_mut().insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rfc_totp_vector_and_replay() {
        let secret = b"12345678901234567890";
        assert_eq!(totp(secret, 1), "287082");
        assert_eq!(verify_code(secret, "287082", 1, None), Some(1));
        assert_eq!(verify_code(secret, "287082", 1, Some(1)), None);
        assert_eq!(verify_code(secret, "12345", 1, None), None);
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
    }
    #[test]
    fn sessions_and_second_factor() {
        let dir = std::env::temp_dir().join(format!("flow-auth-{}", Uuid::new_v4()));
        fs::create_dir(&dir).unwrap();
        let config = Config { digest: digest("long-test-password"), origin: "http://localhost:8780".into(), secure: false,
            path: dir.join("web-auth.json"), state: Arc::new(Mutex::new(AuthState { healthy: true, persisted: Persisted::default(),
                sessions: HashMap::new(), attempts: 0, window: Instant::now() })) };
        let login = |code: String| Input { username: "flow".into(), password: "long-test-password".into(), code };
        assert!(dispatch(&config, "/api/v1/auth/login", &Method::POST, None, Input::default()).is_err());
        let response = dispatch(&config, "/api/v1/auth/login", &Method::POST, None, login(String::new())).unwrap();
        let cookie = response.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.contains("HttpOnly") && cookie.contains("SameSite=Strict"));
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookie.split(';').next().unwrap().parse().unwrap());
        let key = session_key(&headers);
        assert!(key.is_some());
        dispatch(&config, "/api/v1/auth/totp/begin", &Method::POST, key, login(String::new())).unwrap();
        let secret = config.state.lock().unwrap().sessions.get(&key.unwrap()).unwrap().pending.as_ref().unwrap().0.clone();
        let code = totp(&secret, now_step());
        dispatch(&config, "/api/v1/auth/totp/confirm", &Method::POST, key, login(code.clone())).unwrap();
        assert!(dispatch(&config, "/api/v1/auth/login", &Method::POST, None, login(String::new())).is_err());
        assert!(dispatch(&config, "/api/v1/auth/login", &Method::POST, None, login(code)).is_err());
        let stored: Persisted = serde_json::from_slice(&fs::read(&config.path).unwrap()).unwrap();
        assert_eq!(stored.secret, Some(secret));
        dispatch(&config, "/api/v1/auth/logout", &Method::POST, key, Input::default()).unwrap();
        assert!(config.state.lock().unwrap().sessions.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }
}
