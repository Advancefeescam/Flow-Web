//! Same-origin raster artwork endpoint. No arbitrary URL relay, redirects, or active content.
use axum::{body::Body, extract::{Query, State}, http::{header, StatusCode}, response::{IntoResponse, Response}};
use serde::Deserialize;
use std::time::Duration;
use crate::{AppState, streaming::images};

const LIMIT: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
pub struct ImageQuery { u: String }

fn target(raw: &str) -> Option<String> {
    if raw.len() > 8192 { return None; }
    let url = reqwest::Url::parse(raw).ok()?;
    if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some()
        || url.port_or_known_default() != Some(443) || url.fragment().is_some() {
        return None;
    }
    images::image_route_target(images::route_token(), raw)
}

pub fn proxy_url(raw: &str) -> String {
    if target(raw).is_none() { return raw.to_owned(); }
    let mut local = reqwest::Url::parse("http://localhost/media/image").unwrap();
    local.query_pairs_mut().append_pair("u", raw);
    format!("{}?{}", local.path(), local.query().unwrap_or_default())
}

pub async fn get(State(state): State<AppState>, Query(query): Query<ImageQuery>) -> Response {
    let Some(url) = target(&query.u) else { return StatusCode::BAD_REQUEST.into_response(); };
    let Ok(Ok(_permit)) = tokio::time::timeout(Duration::from_secs(3), state.image_requests.acquire()).await
        else { return StatusCode::TOO_MANY_REQUESTS.into_response(); };
    // This client explicitly disables redirects and system proxies. Never forward cookies or auth.
    let Ok(mut upstream) = state.relay.get(url)
        .header(header::ACCEPT, "image/avif,image/webp,image/png,image/jpeg,image/gif")
        .timeout(Duration::from_secs(15)).send().await
        else { return StatusCode::BAD_GATEWAY.into_response(); };
    if !upstream.status().is_success() {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let mime = upstream.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(';').next()).unwrap_or("").trim().to_ascii_lowercase();
    if !matches!(mime.as_str(), "image/jpeg" | "image/png" | "image/webp" | "image/avif" | "image/gif") {
        return StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response();
    }
    if upstream.content_length().is_some_and(|len| len > LIMIT as u64) {
        return StatusCode::PAYLOAD_TOO_LARGE.into_response();
    }
    let mut body = Vec::new();
    loop {
        match upstream.chunk().await {
            Ok(Some(chunk)) if chunk.len() <= LIMIT.saturating_sub(body.len()) => body.extend_from_slice(&chunk),
            Ok(Some(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
            Ok(None) => break,
            Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
        }
    }
    if body.is_empty() { return StatusCode::BAD_GATEWAY.into_response(); }
    Response::builder().header(header::CONTENT_TYPE, mime)
        .header("content-security-policy", "default-src 'none'; sandbox")
        .body(Body::from(body)).unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_https_artwork_hosts_on_standard_port() {
        let good = "https://i.ytimg.com/vi/test/hqdefault.jpg";
        assert!(target(good).is_some());
        assert!(proxy_url(good).starts_with("/media/image?u="));
        for bad in ["http://i.ytimg.com/a", "https://127.0.0.1/a", "https://i.ytimg.com.evil.test/a",
            "https://user@i.ytimg.com/a", "https://i.ytimg.com:1234/a", "https://i.ytimg.com/a#x",
            "https://192.168.1.1/a"] {
            assert!(target(bad).is_none(), "{bad}");
        }
    }
}
