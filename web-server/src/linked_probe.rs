//! An authenticated, non-mutating readiness check. The destination is chosen
//! by the server operator, never by browser RPC input. No admin token, durable
//! device key or user library data is sent by this check.
use serde_json::{Value, json};
use crate::errors::{AppError, AppResult};
use crate::sync2::{keys::member_secrets, sealed::random_bytes, server::{ServerInfo, ServerStore}};

fn server_url(raw: &str) -> AppResult<reqwest::Url> {
    let mut url = reqwest::Url::parse(raw)
        .map_err(|_| AppError::Validation("Invalid FLOW_WEB_SYNC_URL".into()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none()
        || !url.username().is_empty() || url.password().is_some()
        || url.query().is_some() || url.fragment().is_some() {
        return Err(AppError::Validation("FLOW_WEB_SYNC_URL must be an HTTP(S) server URL without credentials, query or fragment".into()));
    }
    if !url.path().ends_with('/') {
        let path = format!("{}/", url.path());
        url.set_path(&path);
    }
    Ok(url)
}

async fn probe(raw: &str) -> AppResult<ServerInfo> {
    let url = server_url(raw)?;
    // /v1/info is public. The ephemeral member is never used to sign in or join.
    let keys = member_secrets(random_bytes(), random_bytes());
    let check = async {
        let store = ServerStore::connect(url.as_str(), keys, None, None).await?;
        store.info().await
    };
    tokio::time::timeout(std::time::Duration::from_secs(12), check).await
        .map_err(|_| AppError::Network("Flow Sync connection timed out".into()))?
        .map_err(|_| AppError::Network("Could not read the Flow Sync server protocol information".into()))
}

pub async fn status() -> AppResult<Value> {
    let Ok(url) = std::env::var("FLOW_WEB_SYNC_URL") else {
        return Ok(json!({"configured":false,"reachable":false,"pairingAvailable":false}));
    };
    if url.trim().is_empty() {
        return Ok(json!({"configured":false,"reachable":false,"pairingAvailable":false}));
    }
    match probe(&url).await {
        Ok(info) => Ok(json!({"configured":true,"reachable":true,"pairingAvailable":false,
            "protocols":info.protocols,"live":info.live})),
        Err(error) => Ok(json!({"configured":true,"reachable":false,"pairingAvailable":false,"error":error.to_string()})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path}};

    #[test]
    fn rejects_secrets_and_non_http_destinations() {
        for raw in ["file:///etc/passwd", "https://user:pass@example.com", "https://example.com/?token=x", "https://example.com/#x"] {
            assert!(server_url(raw).is_err());
        }
        assert_eq!(server_url("http://192.168.1.2:18765/sync").unwrap().as_str(), "http://192.168.1.2:18765/sync/");
    }

    #[tokio::test]
    async fn uses_original_server_info_transport_without_joining() {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/v1/info"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"protocol":["FLOW-SYNC/2"],"live":true})))
            .expect(1).mount(&server).await;
        let info = probe(&server.uri()).await.unwrap();
        assert_eq!(info.protocols, vec!["FLOW-SYNC/2"]);
        assert!(info.live);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].headers.get("authorization").is_none());
    }
}
