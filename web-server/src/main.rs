//! Browser-facing server. Uses Flow's original extraction and streaming modules.
#[path = "../../src-tauri/src/api/mod.rs"] mod api;
#[path = "../../src-tauri/src/appimage.rs"] mod appimage;
#[path = "../../src-tauri/src/errors/mod.rs"] mod errors;
#[path = "../../src-tauri/src/models/mod.rs"] mod models;
#[path = "../../src-tauri/src/streaming/mod.rs"] mod streaming;
#[path = "../../src-tauri/src/security/validation.rs"] pub(crate) mod validation;
#[path = "../../src-tauri/src/services/youtube_service.rs"] mod youtube_service;
mod security { pub(crate) use crate::validation; }
mod rpc;
mod db;
mod library;
mod music;
mod profiles;
#[path = "../../src-tauri/src/services/music_service.rs"] mod music_service;
mod media;
mod auth;

use std::{net::SocketAddr, path::PathBuf, sync::Arc};
use axum::{Router, extract::DefaultBodyLimit, middleware, routing::{get, post}};
use tower_http::services::{ServeDir, ServeFile};
use api::innertube::InnertubeClient;
use streaming::proxy::StreamingManager;
use youtube_service::YoutubeService;

#[derive(Clone)]
struct AppState {
    youtube: YoutubeService,
    music: music_service::MusicService,
    pool: sqlx::SqlitePool,
    writes: Arc<tokio::sync::Mutex<()>>,
    streams: StreamingManager,
    relay: reqwest::Client,
    requests: Arc<tokio::sync::Semaphore>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())
    ).init();
    let bind: SocketAddr = std::env::var("FLOW_WEB_BIND").unwrap_or_else(|_| "127.0.0.1:8780".into()).parse()?;
    let auth = auth::Config::from_env()?;
    let dist = PathBuf::from(std::env::var("FLOW_WEB_DIST").unwrap_or_else(|_| "../dist".into()));
    if !dist.join("index.html").is_file() {
        return Err("Frontend dist/index.html missing; build with pnpm build --mode web".into());
    }
    let data = PathBuf::from(std::env::var("FLOW_WEB_DATA").unwrap_or_else(|_| "./data".into()));
    std::fs::create_dir_all(&data)?;
    let extractor = Arc::new(InnertubeClient::new());
    extractor.keep_identity_in(data.join("visitor.json"));
    let pool = db::open(&data).await?;
    library::load_settings(&pool).await?;
    let music = music_service::MusicService::new(extractor.clone(), pool.clone());
    let youtube = YoutubeService::new(extractor);
    let (streams, listeners) = StreamingManager::new();
    tokio::spawn(streaming::proxy::start_proxy_server(streams.clone(), listeners.media));
    tokio::spawn(streaming::proxy::start_proxy_server(streams.clone(), listeners.images));
    tokio::spawn(streaming::proxy::keep_tidy(streams.clone()));
    streams.set_link_resolver(media::link_resolver(youtube.clone()));
    let relay = reqwest::Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(std::time::Duration::from_secs(5))
        .build()?;
    let state = AppState { youtube, music, pool, writes: Arc::new(tokio::sync::Mutex::new(())), streams, relay, requests: Arc::new(tokio::sync::Semaphore::new(8)) };
    let app = Router::new()
        .route("/api/v1/health", get(|| async { axum::Json(serde_json::json!({"status":"ok","protocol":1,"experimental":true})) }))
        .route("/api/v1/rpc/{command}", post(rpc::handle))
        .route("/api/{*rest}", axum::routing::any(|| async { axum::http::StatusCode::NOT_FOUND }))
        .route("/media/stream/{token}", get(media::stream))
        .route("/media/sabr/{*path}", get(media::sabr))
        .route("/media/{*rest}", axum::routing::any(|| async { axum::http::StatusCode::NOT_FOUND }))
        .fallback_service(ServeDir::new(&dist).fallback(ServeFile::new(dist.join("index.html"))))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(middleware::from_fn_with_state(auth, auth::guard))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "Experimental Flow Web listening");
    axum::serve(listener, app).with_graceful_shutdown(async { let _ = tokio::signal::ctrl_c().await; }).await?;
    Ok(())
}
