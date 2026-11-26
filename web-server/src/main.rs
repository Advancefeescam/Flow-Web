//! Browser-facing server. Uses Flow's original extraction and streaming modules.
#[path = "../../src-tauri/src/api/mod.rs"] mod api;
#[path = "../../src-tauri/src/appimage.rs"] mod appimage;
#[path = "../../src-tauri/src/errors/mod.rs"] mod errors;
#[path = "../../src-tauri/src/models/mod.rs"] mod models;
#[path = "../../src-tauri/src/flow_neuro/mod.rs"] mod flow_neuro;
#[path = "../../src-tauri/src/streaming/mod.rs"] mod streaming;
#[path = "../../src-tauri/src/security/validation.rs"] pub(crate) mod validation;
#[path = "../../src-tauri/src/services/youtube_service.rs"] mod youtube_service;
mod security { pub(crate) use crate::validation; }
mod rpc;
mod db;
mod library;
mod music;
mod profiles;
mod neuro;
mod home;
mod shorts;
mod home_feed;
mod subscription_feed;
#[path = "../../src-tauri/src/services/music_service.rs"] mod music_service;
mod media;
mod hls;
mod auth;
mod artwork;
mod foss;
mod services;

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
    image_requests: Arc<tokio::sync::Semaphore>,
    recommendations: Arc<services::recommendation_service::RecommendationService>,
    home: home::HomeService,
    shorts: Arc<services::shorts_service::ShortsService>,
    blocked_channel_names: Arc<services::blocked_channel_names::BlockedChannelNames>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let result = runtime.block_on(run());
    runtime.shutdown_timeout(std::time::Duration::from_secs(2));
    result
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        if let Ok(mut term) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = term.recv() => {} }
            return;
        }
    }
    let _ = tokio::signal::ctrl_c().await;
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into())
    ).init();
    let bind: SocketAddr = std::env::var("FLOW_WEB_BIND").unwrap_or_else(|_| "127.0.0.1:8780".into()).parse()?;
    let dist = PathBuf::from(std::env::var("FLOW_WEB_DIST").unwrap_or_else(|_| "../dist".into()));
    if !dist.join("index.html").is_file() {
        return Err("Frontend dist/index.html missing; build with pnpm build --mode web".into());
    }
    let data = PathBuf::from(std::env::var("FLOW_WEB_DATA").unwrap_or_else(|_| "./data".into()));
    std::fs::create_dir_all(&data)?;
    let auth = auth::Config::from_env(&data)?;
    let extractor = Arc::new(InnertubeClient::new());
    extractor.keep_identity_in(data.join("visitor.json"));
    let pool = db::open(&data).await?;
    library::load_settings(&pool).await?;
    let brain = flow_neuro::brain_store::BrainStore::load(pool.clone()).await?;
    let neuro_stores = Arc::new(flow_neuro::side_stores::SideStores::load(pool.clone()).await?);
    let recommendations = Arc::new(services::recommendation_service::RecommendationService::new(
        pool.clone(), brain, neuro_stores,
    ));
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
    let state = AppState { youtube, music, pool, writes: Arc::new(tokio::sync::Mutex::new(())), streams, relay, requests: Arc::new(tokio::sync::Semaphore::new(8)), image_requests: Arc::new(tokio::sync::Semaphore::new(8)), recommendations, home: home::HomeService::new(), shorts: Arc::new(services::shorts_service::ShortsService::new()), blocked_channel_names: Arc::new(services::blocked_channel_names::BlockedChannelNames::default()) };
    let app = Router::new()
        .route("/api/v1/health", get(|| async { axum::Json(serde_json::json!({"status":"ok","protocol":1,"experimental":true})) }))
        .route("/api/v1/rpc/{command}", post(rpc::handle))
        .route("/api/{*rest}", axum::routing::any(|| async { axum::http::StatusCode::NOT_FOUND }))
        .route("/media/image", get(artwork::get))
        .route("/media/stream/{token}", get(media::stream))
        .route("/media/hls/{token}", get(hls::relay))
        .route("/media/sabr/{*path}", get(media::sabr))
        .route("/media/{*rest}", axum::routing::any(|| async { axum::http::StatusCode::NOT_FOUND }))
        .fallback_service(ServeDir::new(&dist).fallback(ServeFile::new(dist.join("index.html"))))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(middleware::from_fn_with_state(auth, auth::guard))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "Experimental Flow Web listening");
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let mut server = tokio::spawn(async move {
        axum::serve(listener, app).with_graceful_shutdown(async { let _ = stopped.await; }).await
    });
    tokio::select! {
        result = &mut server => { result??; }
        _ = shutdown_signal() => {
            tracing::info!("Stopping Flow Web (up to 5 seconds for active requests)");
            let _ = stop.send(());
            match tokio::time::timeout(std::time::Duration::from_secs(5), &mut server).await {
                Ok(result) => { result??; }
                Err(_) => {
                    tracing::warn!("Active connections did not finish; stopping server");
                    server.abort();
                }
            }
        }
    }
    Ok(())
}
