use axum::{
    extract::{Request, State},
    http::{Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use subtle::ConstantTimeEq;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct Config { digest: [u8; 32], origin: String }
impl Config {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let password = std::env::var("FLOW_WEB_PASSWORD")?;
        if password.len() < 16 { return Err("FLOW_WEB_PASSWORD must have at least 16 bytes".into()); }
        let raw = std::env::var("FLOW_WEB_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:8780".into());
        let url = reqwest::Url::parse(&raw)?;
        if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
            || url.path() != "/" || url.query().is_some() || url.fragment().is_some()
            || !url.username().is_empty() || url.password().is_some() {
            return Err("FLOW_WEB_ORIGIN must be an HTTP(S) origin without a path".into());
        }
        Ok(Self { digest: Sha256::digest(format!("flow:{password}")).into(), origin: url.origin().ascii_serialization() })
    }

    fn valid(&self, value: Option<&str>) -> bool {
        let Some(decoded) = value.and_then(|s| s.strip_prefix("Basic ")).and_then(|s| STANDARD.decode(s).ok()) else { return false; };
        let digest: [u8; 32] = Sha256::digest(decoded).into();
        bool::from(self.digest.ct_eq(&digest))
    }
}

pub async fn guard(State(config): State<Config>, request: Request, next: Next) -> Response {
    let h = request.headers();
    if h.get("sec-fetch-site").and_then(|v| v.to_str().ok()) == Some("cross-site")
        || (request.method() == Method::POST && h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) != Some(config.origin.as_str())) {
        return (StatusCode::FORBIDDEN, "Cross-origin request refused").into_response();
    }
    if !config.valid(h.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok())) {
        return (StatusCode::UNAUTHORIZED, [(header::WWW_AUTHENTICATE, "Basic realm=\"Flow Web\", charset=\"UTF-8\"")], "Authentication required").into_response();
    }
    let mut response = next.run(request).await;
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
    fn credentials_are_checked() {
        let config = Config { digest: Sha256::digest("flow:a-long-test-password").into(), origin: "http://localhost:8780".into() };
        assert!(config.valid(Some(&format!("Basic {}", STANDARD.encode("flow:a-long-test-password")))));
        assert!(!config.valid(Some(&format!("Basic {}", STANDARD.encode("flow:wrong")))));
        assert!(!config.valid(None));
    }
}
