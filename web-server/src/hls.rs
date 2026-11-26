use crate::{AppState, media::allowed_remote, streaming::proxy::StreamSessionKind};
use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, Method, StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;

const MAX_PLAYLIST_BYTES: usize = 2 * 1024 * 1024;
const MAX_REDIRECTS: usize = 3;

fn safe_target(base: &reqwest::Url, raw: &str) -> Option<reqwest::Url> {
    let url = base.join(raw).ok()?;
    allowed_remote(url.as_str()).then_some(url)
}

fn proxy_uri(state: &AppState, url: &str, user_agent: &str) -> String {
    let token = uuid::Uuid::new_v4().to_string();
    state.streams.register_session(
        token.clone(),
        url.into(),
        "application/octet-stream".into(),
        user_agent.into(),
    );
    format!("/media/hls/{token}")
}

fn rewrite_uri(
    base: &reqwest::Url,
    raw: &str,
    register: &mut impl FnMut(&str) -> String,
) -> Option<String> {
    let url = safe_target(base, raw)?;
    Some(register(url.as_str()))
}

fn rewrite_playlist(
    base: &reqwest::Url,
    body: &str,
    mut register: impl FnMut(&str) -> String,
) -> Option<String> {
    if !body.starts_with("#EXTM3U") {
        return None;
    }
    let mut rewritten = String::with_capacity(body.len());
    for line in body.lines() {
        if line.starts_with('#') {
            if let Some((before, rest)) = line.split_once("URI=\"") {
                let (raw, after) = rest.split_once('"')?;
                let url = rewrite_uri(base, raw, &mut register)?;
                rewritten.push_str(before);
                rewritten.push_str("URI=\"");
                rewritten.push_str(&url);
                rewritten.push('"');
                rewritten.push_str(after);
            } else {
                rewritten.push_str(line);
            }
        } else if !line.trim().is_empty() {
            rewritten.push_str(&rewrite_uri(base, line.trim(), &mut register)?);
        }
        rewritten.push('\n');
    }
    Some(rewritten)
}

pub async fn relay(
    State(state): State<AppState>,
    Path(token): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Response {
    if uuid::Uuid::parse_str(&token).is_err() {
        return StatusCode::NOT_FOUND.into_response();
    }
    let Some(session) = state.streams.get_session(&token) else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let StreamSessionKind::Remote { remote_url } = session.kind else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !allowed_remote(&remote_url) {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let Ok(mut base) = reqwest::Url::parse(&remote_url) else {
        return StatusCode::BAD_GATEWAY.into_response();
    };
    let known_playlist = session.content_type.contains("mpegurl") || base.path().ends_with(".m3u8");
    let mut redirects = 0;
    let upstream = loop {
        let mut request = state
            .relay
            .request(method.clone(), base.clone())
            .header(header::USER_AGENT, &session.user_agent);
        if !known_playlist {
            if let Some(range) = headers.get(header::RANGE) {
                request = request.header(header::RANGE, range);
            }
        }
        let Ok(response) = request.send().await else {
            return StatusCode::BAD_GATEWAY.into_response();
        };
        if !response.status().is_redirection() {
            break response;
        }
        if redirects == MAX_REDIRECTS {
            return StatusCode::BAD_GATEWAY.into_response();
        }
        let Some(next) = response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| safe_target(&base, value))
        else {
            return StatusCode::BAD_GATEWAY.into_response();
        };
        base = next;
        redirects += 1;
    };
    let status = upstream.status();
    let content_type = upstream
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let playlist = session.content_type.contains("mpegurl")
        || content_type.to_ascii_lowercase().contains("mpegurl")
        || base.path().ends_with(".m3u8");
    let mut response = Response::builder().status(status).header(
        header::CONTENT_TYPE,
        if playlist {
            "application/vnd.apple.mpegurl"
        } else if content_type.is_empty() {
            "application/octet-stream"
        } else {
            &content_type
        },
    );
    if playlist && method == Method::HEAD {
        return response
            .body(Body::empty())
            .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response());
    }
    if playlist && status.is_success() && method != Method::HEAD {
        let mut body = Vec::new();
        let mut stream = upstream.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let Ok(chunk) = chunk else {
                return StatusCode::BAD_GATEWAY.into_response();
            };
            if body.len() + chunk.len() > MAX_PLAYLIST_BYTES {
                return StatusCode::BAD_GATEWAY.into_response();
            }
            body.extend_from_slice(&chunk);
        }
        let Ok(body) = std::str::from_utf8(&body) else {
            return StatusCode::BAD_GATEWAY.into_response();
        };
        let Some(body) = rewrite_playlist(&base, body, |url| {
            proxy_uri(&state, url, &session.user_agent)
        }) else {
            return StatusCode::BAD_GATEWAY.into_response();
        };
        return response
            .body(Body::from(body))
            .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response());
    }
    for name in [
        header::CONTENT_LENGTH,
        header::CONTENT_RANGE,
        header::ACCEPT_RANGES,
    ] {
        if let Some(value) = upstream.headers().get(&name) {
            response = response.header(name, value);
        }
    }
    response
        .body(Body::from_stream(upstream.bytes_stream()))
        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_targets_must_stay_on_allowed_hosts() {
        let base =
            reqwest::Url::parse("https://manifest.googlevideo.com/api/manifest/hls_playlist/abc")
                .unwrap();
        for raw in [
            "http://127.0.0.1/private",
            "https://googlevideo.com.evil.test/key",
            "file:///etc/passwd",
        ] {
            assert!(!allowed_remote(base.join(raw).unwrap().as_str()));
        }
        assert!(allowed_remote(base.join("../segment.ts").unwrap().as_str()));
        assert!(safe_target(&base, "https://rr2.googlevideo.com/next").is_some());
        assert!(safe_target(&base, "http://127.0.0.1/private").is_none());
    }

    #[test]
    fn rewrites_playlist_variants_segments_and_keys() {
        let base =
            reqwest::Url::parse("https://manifest.googlevideo.com/live/master.m3u8").unwrap();
        let playlist = "#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI=\"key.bin\"\n#EXT-X-STREAM-INF:BANDWIDTH=1000\nlevel.m3u8\nsegment.ts\n";
        let mut targets = Vec::new();
        let body = rewrite_playlist(&base, playlist, |url| {
            targets.push(url.to_string());
            format!("/media/hls/{}", targets.len())
        })
        .unwrap();
        assert!(body.contains("URI=\"/media/hls/1\""));
        assert!(body.contains("/media/hls/2\n/media/hls/3"));
        assert_eq!(
            targets,
            [
                "https://manifest.googlevideo.com/live/key.bin",
                "https://manifest.googlevideo.com/live/level.m3u8",
                "https://manifest.googlevideo.com/live/segment.ts",
            ]
        );
        assert!(
            rewrite_playlist(&base, "#EXTM3U\nhttp://127.0.0.1/private\n", |_| "unused"
                .into())
            .is_none()
        );
    }
}
