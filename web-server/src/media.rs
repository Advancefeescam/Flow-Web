use std::sync::Arc;
use axum::{body::Body, extract::{Path, State}, http::{HeaderMap, Method, StatusCode, header}, response::{IntoResponse, Response}};
use crate::{
    AppState,
    api::innertube::core::clients,
    errors::{AppError, AppResult, ErrorResponse},
    models::video::{StreamInfo, VideoSummary},
    streaming::{proxy::StreamingManager, swap::{LinkResolver, StreamLink}},
    validation::{validate_channel_id, validate_video_id},
    youtube_service::YoutubeService,
};

fn stream_user_agent(info: &StreamInfo) -> String {
    info.expires_at
        .split('|')
        .nth(1)
        .map_or_else(|| clients::VISIONOS.user_agent.to_string(), str::to_string)
}

/// Every googlevideo link in `info` the player may fetch, before the proxy takes them over.
fn stream_links(info: &StreamInfo) -> Vec<StreamLink> {
    let user_agent = stream_user_agent(info);
    let progressive = std::iter::once(info.local_url.clone());
    let variants = info
        .variants
        .iter()
        .map(|variant| variant.local_url.clone());
    let audio = info.audio_tracks.iter().map(|track| {
        (
            track.local_url.clone(),
            track
                .user_agent
                .clone()
                .unwrap_or_else(|| user_agent.clone()),
        )
    });
    progressive
        .chain(variants)
        .map(|url| (url, user_agent.clone()))
        .chain(audio)
        .filter(|(url, _)| allowed_remote(url))
        .map(|(url, user_agent)| StreamLink::new(url, user_agent))
        .collect()
}

/// Fresh links for a video one of whose links googlevideo refused mid-play. The refusal has
/// already demoted the walled client, so this walks the ladder as a new video would.
#[must_use]
pub fn link_resolver(youtube_service: YoutubeService) -> LinkResolver {
    Arc::new(move |video_id: String| {
        let youtube_service = youtube_service.clone();
        Box::pin(async move {
            let info = youtube_service.get_stream_info(&video_id, true).await;
            info.ok().map(|info| stream_links(&info))
        })
    })
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
#[tracing::instrument(skip_all, fields(video_id = %video_id))]
pub async fn get_stream_info(
    video_id: String,
    refresh: Option<bool>,
    youtube_service: State<'_, YoutubeService>,
    streaming_manager: State<'_, StreamingManager>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<StreamInfo, ErrorResponse> {
    validate_video_id(&video_id).map_err(ErrorResponse::from)?;

    let refresh = refresh.unwrap_or(false);
    // Feeds stop offering a video refused for good, and offer it again once it plays.
    let mut stream_info = match youtube_service.get_stream_info(&video_id, refresh).await {
        Ok(info) => {
            let _ = crate::db::unplayable_videos::clear(&pool, &video_id).await;
            info
        }
        Err(error) => {
            let lasting = error.is_lasting_refusal();
            let response = ErrorResponse::from(error);
            if lasting {
                let now = chrono::Utc::now().timestamp_millis();
                let _ =
                    crate::db::unplayable_videos::mark(&pool, &video_id, &response.kind, now).await;
            }
            return Err(response);
        }
    };

    let token = uuid::Uuid::new_v4().to_string();

    let clean_expires_at = stream_info
        .expires_at
        .split('|')
        .next()
        .unwrap_or_default()
        .to_string();
    let dynamic_user_agent = stream_user_agent(&stream_info);
    let proxy_port = streaming_manager.get_port();

    if let Some(manifest_url) = stream_info
        .dash_manifest_url
        .clone()
        .filter(|value| !value.is_empty())
    {
        let dash_token = uuid::Uuid::new_v4().to_string();
        streaming_manager.register_session(
            dash_token.clone(),
            manifest_url.clone(),
            "application/dash+xml".to_string(),
            dynamic_user_agent.clone(),
        );

        let mut local_manifest_url =
            reqwest::Url::parse(&format!("http://127.0.0.1:{proxy_port}/proxy/{dash_token}"))
                .expect("failed to build local DASH proxy URL");
        local_manifest_url
            .query_pairs_mut()
            .append_pair("url", &manifest_url);
        stream_info.dash_manifest_url = Some(local_manifest_url.to_string());
    }

    if let Some(manifest_url) = stream_info
        .hls_manifest_url
        .clone()
        .filter(|value| !value.is_empty())
    {
        let hls_token = uuid::Uuid::new_v4().to_string();
        streaming_manager.register_session(
            hls_token.clone(),
            manifest_url.clone(),
            "application/vnd.apple.mpegurl".to_string(),
            dynamic_user_agent.clone(),
        );

        let mut local_manifest_url =
            reqwest::Url::parse(&format!("http://127.0.0.1:{proxy_port}/proxy/{hls_token}"))
                .expect("failed to build local HLS proxy URL");
        local_manifest_url
            .query_pairs_mut()
            .append_pair("url", &manifest_url);
        stream_info.hls_manifest_url = Some(local_manifest_url.to_string());
    }

    // A SABR-only or manifest-only response has no progressive URL; registering
    // an empty one only hands the player a route that fails on first request.
    let has_progressive_url = !stream_info.local_url.is_empty();
    if has_progressive_url {
        streaming_manager.register_video_stream(
            &video_id,
            token.clone(),
            stream_info.local_url.clone(),
            "video/mp4".to_string(),
            dynamic_user_agent.clone(),
        );
    }

    for variant in &mut stream_info.variants {
        if !variant.is_playable || variant.local_url.is_empty() {
            continue;
        }

        let variant_token = uuid::Uuid::new_v4().to_string();
        let mime_type = variant
            .mime_type
            .as_deref()
            .and_then(|mime| mime.split(';').next())
            .unwrap_or("video/mp4")
            .to_string();

        streaming_manager.register_video_stream(
            &video_id,
            variant_token.clone(),
            variant.local_url.clone(),
            mime_type,
            dynamic_user_agent.clone(),
        );

        variant.local_url = format!("http://127.0.0.1:{}/stream/{}", proxy_port, variant_token);
    }

    for caption in &mut stream_info.captions {
        let caption_token = uuid::Uuid::new_v4().to_string();

        streaming_manager.register_session(
            caption_token.clone(),
            caption.url.clone(),
            "text/vtt; charset=utf-8".to_string(),
            crate::api::http::BROWSER_USER_AGENT.to_string(),
        );

        caption.url = format!("http://127.0.0.1:{}/stream/{}", proxy_port, caption_token);
    }

    for audio_track in &mut stream_info.audio_tracks {
        if audio_track.local_url.is_empty() {
            continue;
        }

        let audio_token = uuid::Uuid::new_v4().to_string();
        let mime_type = audio_track
            .mime_type
            .as_deref()
            .and_then(|mime| mime.split(';').next())
            .unwrap_or("audio/mp4")
            .to_string();

        streaming_manager.register_video_stream(
            &video_id,
            audio_token.clone(),
            audio_track.local_url.clone(),
            mime_type,
            audio_track
                .user_agent
                .clone()
                .unwrap_or_else(|| dynamic_user_agent.clone()),
        );

        audio_track.local_url = format!("http://127.0.0.1:{}/stream/{}", proxy_port, audio_token);
    }

    if let Some(descriptor) = stream_info.sabr_descriptor.take() {
        if refresh {
            streaming_manager.sabr().discard_video(&descriptor.video_id);
        }
        let session_id = streaming_manager
            .sabr()
            .prepare(descriptor, crate::streaming::sabr::CodecSupport::default());
        if let Some(sabr) = stream_info.sabr.as_mut() {
            sabr.manifest_url = Some(format!(
                "http://127.0.0.1:{}/sabr/{}/manifest.mpd",
                proxy_port, session_id
            ));
        }
        info!(video_id = %video_id, session = %session_id, "Prepared lazy SABR session (debug/future use)");
    }

    if stream_info
        .dash_manifest_url
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        if let Some(manifest) = build_synthetic_dash_manifest(&stream_info) {
            let manifest_token = uuid::Uuid::new_v4().to_string();
            streaming_manager.register_inline_session(
                manifest_token.clone(),
                manifest.into_bytes(),
                "application/dash+xml".to_string(),
            );
            stream_info.dash_manifest_url = Some(format!(
                "http://127.0.0.1:{}/stream/{}",
                proxy_port, manifest_token
            ));
            info!(video_id = %video_id, "Generated synthetic DASH manifest for fallback playback");
        }
    }

    // Rewrite to clean expires_at for frontend consumption
    stream_info.expires_at = clean_expires_at;

    // Rewrite to local loopback URL
    if has_progressive_url {
        stream_info.local_url = format!("http://127.0.0.1:{proxy_port}/stream/{token}");
    }

    Ok(stream_info)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_sabr_debug_state(
    session_id: String,
    streaming_manager: State<'_, StreamingManager>,
) -> Result<Option<crate::streaming::sabr::engine::SabrDebugState>, ErrorResponse> {
    match streaming_manager.sabr().get(&session_id) {
        Some(handle) => Ok(Some(handle.engine.debug_state().await)),
        None => Ok(None),
    }
}

/// A channel's landing page: header, tabs and Home in one request.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_channel_page(
    channel_id: String,
    youtube_service: State<'_, YoutubeService>,
) -> Result<ChannelPage, ErrorResponse> {
    validate_channel_id(&channel_id).map_err(ErrorResponse::from)?;
    youtube_service
        .get_channel_page(&channel_id)
        .await
        .map_err(ErrorResponse::from)
}

/// A channel's About panel, by the token its header carries.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_channel_about(
    token: String,
    youtube_service: State<'_, YoutubeService>,
) -> Result<ChannelAbout, ErrorResponse> {
    youtube_service
        .get_channel_about(&token)
        .await
        .map_err(ErrorResponse::from)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_channel_details(
    channel_id: String,
    youtube_service: State<'_, YoutubeService>,
) -> Result<ChannelDetails, ErrorResponse> {
    validate_channel_id(&channel_id).map_err(ErrorResponse::from)?;

    youtube_service
        .get_channel_details(&channel_id)
        .await
        .map_err(ErrorResponse::from)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_channel_tab(
    channel_id: String,
    params: Option<String>,
    page_token: Option<String>,
    query: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<ChannelTabResponse, ErrorResponse> {
    validate_channel_id(&channel_id).map_err(ErrorResponse::from)?;
    if let Some(token) = page_token.as_deref() {
        validate_page_token(token).map_err(ErrorResponse::from)?;
    }
    if let Some(ref q) = query {
        validate_search_query(q).map_err(ErrorResponse::from)?;
    }

    youtube_service
        .get_channel_tab(&channel_id, params, page_token, query)
        .await
        .map_err(ErrorResponse::from)
}
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_playlist_details(
    playlist_id: String,
    page_token: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<PlaylistDetailsResponse, ErrorResponse> {
    if playlist_id.trim().is_empty() {
        return Err(ErrorResponse::from(crate::errors::AppError::Validation(
            "Playlist ID cannot be empty".into(),
        )));
    }
    if let Some(token) = page_token.as_deref() {
        validate_page_token(token).map_err(ErrorResponse::from)?;
    }

    youtube_service
        .get_playlist_details(&playlist_id, page_token)
        .await
        .map_err(ErrorResponse::from)
}

/// A Mix (`RD…`, not `RDCLAK…`) as a playlist, read from its watch queue because
/// the playlist browse refuses Mixes. `video_id` seeds a Mix whose id names no video.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_mix_playlist(
    playlist_id: String,
    video_id: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<PlaylistDetailsResponse, ErrorResponse> {
    validate_playlist_id(&playlist_id).map_err(ErrorResponse::from)?;
    if let Some(id) = video_id.as_deref() {
        validate_video_id(id).map_err(ErrorResponse::from)?;
    }

    youtube_service
        .get_mix_playlist(playlist_id.trim(), video_id.as_deref().map(str::trim))
        .await
        .map_err(ErrorResponse::from)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_comments(
    video_id: String,
    page_token: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<CommentsResponse, ErrorResponse> {
    if let Some(token) = page_token.as_deref() {
        validate_page_token(token).map_err(ErrorResponse::from)?;
    } else {
        validate_video_id(&video_id).map_err(ErrorResponse::from)?;
    }

    youtube_service
        .get_comments(&video_id, page_token)
        .await
        .map_err(ErrorResponse::from)
}

/// A Short's comments, through its reel's comment panel when it has one.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_short_comments(
    video_id: String,
    page_token: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<CommentsResponse, ErrorResponse> {
    validate_video_id(&video_id).map_err(ErrorResponse::from)?;
    if let Some(token) = page_token.as_deref() {
        validate_page_token(token).map_err(ErrorResponse::from)?;
    }
    youtube_service
        .get_short_comments(&video_id, page_token)
        .await
        .map_err(ErrorResponse::from)
}

/// YouTube's Timed list for a video, from the token of its comment section's Newest order.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_timed_comments(
    video_id: String,
    section_token: String,
    youtube_service: State<'_, YoutubeService>,
) -> Result<Vec<Comment>, ErrorResponse> {
    validate_video_id(&video_id).map_err(ErrorResponse::from)?;
    validate_page_token(&section_token).map_err(ErrorResponse::from)?;
    youtube_service
        .get_timed_comments(&video_id, &section_token)
        .await
        .map_err(ErrorResponse::from)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_post_comments(
    post_id: String,
    params: Option<String>,
    page_token: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<CommentsResponse, ErrorResponse> {
    if let Some(token) = page_token.as_deref() {
        validate_page_token(token).map_err(ErrorResponse::from)?;
    } else {
        validate_browse_id(&post_id).map_err(ErrorResponse::from)?;
        if let Some(post_params) = params.as_deref() {
            validate_page_token(post_params).map_err(ErrorResponse::from)?;
        }
    }

    youtube_service
        .get_post_comments(&post_id, params, page_token)
        .await
        .map_err(ErrorResponse::from)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_live_chat(
    video_id: String,
    continuation: Option<String>,
    replay_offset_ms: Option<u64>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<crate::models::live_chat::LiveChatResponse, ErrorResponse> {
    validate_video_id(&video_id).map_err(ErrorResponse::from)?;

    youtube_service
        .get_live_chat(&video_id, continuation, replay_offset_ms)
        .await
        .map_err(ErrorResponse::from)
}

/// One of Explore's destinations (`live`, `gaming`, `music`, `news`, `sports`,
/// `learning`, `fashion`) in `region`.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_explore_destination(
    destination: String,
    region: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<ExplorePage, ErrorResponse> {
    youtube_service
        .get_explore_destination(&destination, region.as_deref())
        .await
        .map_err(ErrorResponse::from)
}

/// A destination's tab, by the target its page named.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_explore_page(
    target: ExploreTarget,
    region: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<ExplorePage, ErrorResponse> {
    youtube_service
        .get_explore_page(
            &target.browse_id,
            target.params.as_deref(),
            region.as_deref(),
        )
        .await
        .map_err(ErrorResponse::from)
}

/// A shelf's full grid: the first page from its target, later ones from a token.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_explore_grid(
    target: Option<ExploreTarget>,
    continuation: Option<String>,
    region: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<ExploreGrid, ErrorResponse> {
    youtube_service
        .get_explore_grid(target.as_ref(), continuation.as_deref(), region.as_deref())
        .await
        .map_err(ErrorResponse::from)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_search_suggestions(
    query: String,
    region: Option<String>,
    youtube_service: State<'_, YoutubeService>,
) -> Result<Vec<SearchSuggestion>, ErrorResponse> {
    youtube_service
        .get_search_suggestions(&query, region.as_deref())
        .await
        .map_err(ErrorResponse::from)
}

/// Opens the suggest connection while the search box is empty. Never fails:
/// a connection that doesn't open just leaves the first keystroke to open it.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn warm_search_suggestions(
    youtube_service: State<'_, YoutubeService>,
) -> Result<(), ErrorResponse> {
    youtube_service.warm_search_suggestions().await;
    Ok(())
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn parse_subscription_export(
    data: String,
    youtube_service: State<'_, YoutubeService>,
) -> Result<Vec<(String, String)>, ErrorResponse> {
    youtube_service
        .parse_subscription_export(&data)
        .map_err(ErrorResponse::from)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_sponsorblock_segments(
    video_id: String,
    server_url: Option<String>,
) -> Result<serde_json::Value, ErrorResponse> {
    fetch_sponsorblock_segments(&video_id, server_url.as_deref())
        .await
        .map_err(ErrorResponse::from)
}

pub(crate) async fn fetch_sponsorblock_segments(
    video_id: &str,
    server_url: Option<&str>,
) -> crate::errors::AppResult<serde_json::Value> {
    validate_video_id(video_id)?;

    let base_url = normalize_sponsorblock_server_url(server_url)?;
    let url = format!(
        "{base_url}/api/skipSegments?videoID={video_id}&categories=[\"sponsor\",\"intro\",\"outro\",\"selfpromo\",\"interaction\",\"filler\",\"music_offtopic\",\"preview\",\"exclusive_access\",\"hook\",\"poi_highlight\"]&actionTypes=[\"skip\",\"mute\",\"full\",\"poi\"]"
    );

    let res = crate::api::http::shared_client()
        .get(&url)
        .header(reqwest::header::USER_AGENT, SPONSORBLOCK_USER_AGENT)
        .timeout(SPONSORBLOCK_TIMEOUT)
        .send()
        .await
        .map_err(|e| crate::errors::AppError::Extractor(format!("Network error: {e}")))?;

    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(serde_json::json!([]));
    }

    if !status.is_success() {
        return Err(crate::errors::AppError::Extractor(format!(
            "SponsorBlock server returned status {status}"
        )));
    }

    res.json::<serde_json::Value>()
        .await
        .map_err(|e| crate::errors::AppError::Extractor(format!("JSON parse error: {e}")))
}

/// A part of a music video marked as having no song playing, in seconds. A
/// `video_duration_seconds` of zero means the voter's video length wasn't recorded.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NonMusicSegment {
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub video_duration_seconds: f64,
}

#[derive(serde::Deserialize)]
struct HashedVideo {
    #[serde(rename = "videoID")]
    video_id: String,
    #[serde(default)]
    segments: Vec<HashedSegment>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct HashedSegment {
    #[serde(default)]
    category: String,
    #[serde(default)]
    action_type: String,
    #[serde(default)]
    segment: Vec<f64>,
    #[serde(default)]
    video_duration: f64,
}

const NON_MUSIC_CATEGORY: &str = "music_offtopic";
const NON_MUSIC_HASH_PREFIX_CHARS: usize = 4;
const NON_MUSIC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(8);

/// The parts of a music video marked as non-music, for fitting its lyrics. This
/// runs for each song played rather than for videos someone chose to use
/// SponsorBlock on, so only a short prefix of the id's hash is sent and the
/// server can't tell which video is playing. No segments comes back as a 404.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_non_music_segments(
    video_id: String,
    server_url: Option<String>,
) -> Result<Vec<NonMusicSegment>, ErrorResponse> {
    fetch_non_music_segments(&video_id, server_url.as_deref())
        .await
        .map_err(ErrorResponse::from)
}

async fn fetch_non_music_segments(
    video_id: &str,
    server_url: Option<&str>,
) -> crate::errors::AppResult<Vec<NonMusicSegment>> {
    validate_video_id(video_id)?;
    let base_url = normalize_sponsorblock_server_url(server_url)?;
    let res = crate::api::http::shared_client()
        .get(non_music_segments_url(&base_url, video_id))
        .header(reqwest::header::USER_AGENT, SPONSORBLOCK_USER_AGENT)
        .timeout(NON_MUSIC_TIMEOUT)
        .send()
        .await
        .map_err(|e| crate::errors::AppError::Extractor(format!("Network error: {e}")))?;
    let status = res.status();
    if status == reqwest::StatusCode::NOT_FOUND {
        return Ok(Vec::new());
    }
    if !status.is_success() {
        return Err(crate::errors::AppError::Extractor(format!(
            "SponsorBlock server returned status {status}"
        )));
    }
    let body = res
        .text()
        .await
        .map_err(|e| crate::errors::AppError::Extractor(format!("Network error: {e}")))?;
    parse_hashed_segments(&body, video_id)
}

fn non_music_segments_url(base_url: &str, video_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = hex::encode(Sha256::digest(video_id.as_bytes()));
    format!(
        "{base_url}/api/skipSegments/{}?categories=[\"{NON_MUSIC_CATEGORY}\"]&actionTypes=[\"skip\"]",
        &hash[..NON_MUSIC_HASH_PREFIX_CHARS]
    )
}

/// The segments of `video_id` among the videos sharing its hash prefix.
fn parse_hashed_segments(
    json: &str,
    video_id: &str,
) -> crate::errors::AppResult<Vec<NonMusicSegment>> {
    let videos: Vec<HashedVideo> = serde_json::from_str(json)
        .map_err(|e| crate::errors::AppError::Extractor(format!("JSON parse error: {e}")))?;
    Ok(videos
        .into_iter()
        .find(|video| video.video_id == video_id)
        .map(|video| video.segments)
        .unwrap_or_default()
        .into_iter()
        .filter(|s| s.category == NON_MUSIC_CATEGORY && s.action_type == "skip")
        .filter_map(|s| match s.segment.as_slice() {
            [start, end] => Some(NonMusicSegment {
                start_seconds: *start,
                end_seconds: *end,
                video_duration_seconds: s.video_duration,
            }),
            _ => None,
        })
        .collect())
}

fn normalize_sponsorblock_server_url(server_url: Option<&str>) -> crate::errors::AppResult<String> {
    let raw = server_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("https://sponsor.ajay.app");
    let parsed = reqwest::Url::parse(raw).map_err(|_| {
        crate::errors::AppError::Validation("Invalid SponsorBlock server URL".into())
    })?;

    match parsed.scheme() {
        "http" | "https" => {}
        _ => {
            return Err(crate::errors::AppError::Validation(
                "SponsorBlock server URL must use http or https".into(),
            ));
        }
    }

    let origin = parsed.origin().unicode_serialization();
    if origin == "null" {
        return Err(crate::errors::AppError::Validation(
            "SponsorBlock server URL must include an origin".into(),
        ));
    }
    Ok(origin)
}

const SPONSORBLOCK_USER_AGENT: &str = "FlowDesktop/1.0";
const SPONSORBLOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

const SPONSORBLOCK_SUBMIT_CATEGORIES: [&str; 10] = [
    "sponsor",
    "intro",
    "outro",
    "selfpromo",
    "interaction",
    "music_offtopic",
    "filler",
    "preview",
    "exclusive_access",
    "hook",
];

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn submit_sponsorblock_segment(
    video_id: String,
    start_time: f64,
    end_time: f64,
    category: String,
    user_id: String,
    server_url: Option<String>,
) -> Result<(), ErrorResponse> {
    submit_sponsorblock_segment_inner(
        &video_id,
        start_time,
        end_time,
        &category,
        &user_id,
        server_url.as_deref(),
    )
    .await
    .map_err(ErrorResponse::from)
}

async fn submit_sponsorblock_segment_inner(
    video_id: &str,
    start_time: f64,
    end_time: f64,
    category: &str,
    user_id: &str,
    server_url: Option<&str>,
) -> crate::errors::AppResult<()> {
    validate_video_id(video_id)?;

    if !SPONSORBLOCK_SUBMIT_CATEGORIES.contains(&category) {
        return Err(crate::errors::AppError::Validation(
            "Unsupported SponsorBlock category".into(),
        ));
    }

    if !start_time.is_finite()
        || !end_time.is_finite()
        || start_time < 0.0
        || end_time <= start_time
    {
        return Err(crate::errors::AppError::Validation(
            "Segment end time must be greater than its start time".into(),
        ));
    }

    // SponsorBlock derives a public contributor ID by hashing this private one, so it
    // must be a long, non-trivial string. Never log it — it is user-identifying.
    let user_id = user_id.trim();
    if user_id.len() < 8 {
        return Err(crate::errors::AppError::Validation(
            "A valid SponsorBlock user ID is required to submit segments".into(),
        ));
    }

    let base_url = normalize_sponsorblock_server_url(server_url)?;

    let mut url = reqwest::Url::parse(&format!("{base_url}/api/skipSegments")).map_err(|_| {
        crate::errors::AppError::Validation("Invalid SponsorBlock server URL".into())
    })?;
    url.query_pairs_mut()
        .append_pair("videoID", video_id)
        .append_pair("startTime", &start_time.to_string())
        .append_pair("endTime", &end_time.to_string())
        .append_pair("category", category)
        .append_pair("userID", user_id)
        .append_pair("userAgent", "FlowDesktop/1.0");

    let res = crate::api::http::shared_client()
        .post(url)
        .header(reqwest::header::USER_AGENT, SPONSORBLOCK_USER_AGENT)
        .timeout(SPONSORBLOCK_TIMEOUT)
        .send()
        .await
        .map_err(|e| crate::errors::AppError::Extractor(format!("Network error: {e}")))?;

    let status = res.status();
    if status.is_success() {
        return Ok(());
    }

    let body = res.text().await.unwrap_or_default();
    let detail = body.trim();
    let message = if detail.is_empty() {
        format!("SponsorBlock rejected the submission (status {status})")
    } else {
        format!("SponsorBlock rejected the submission: {detail}")
    };
    Err(crate::errors::AppError::Extractor(message))
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_dearrow_override(
    video_id: String,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<Option<crate::api::dearrow::DeArrowOverride>, ErrorResponse> {
    validate_video_id(&video_id).map_err(ErrorResponse::from)?;

    // 1. Try to fetch from local dearrow_cache first.
    if let Ok(Some(cached)) = crate::db::cache::get_cached_dearrow(&pool, &video_id).await {
        if cached.title.is_none() && cached.thumbnail_url.is_none() {
            return Ok(None);
        }
        return Ok(Some(cached));
    }

    // 2. Fetch from SponsorBlock/DeArrow API
    match crate::api::dearrow::fetch_dearrow_override_api(&video_id).await {
        Ok(Some(override_data)) => {
            let _ = crate::db::cache::cache_dearrow(&pool, &video_id, &override_data).await;
            Ok(Some(override_data))
        }
        Ok(None) => {
            let empty_override = crate::api::dearrow::DeArrowOverride {
                random_frame_url: None,
                title: None,
                thumbnail_url: None,
            };
            let _ = crate::db::cache::cache_dearrow(&pool, &video_id, &empty_override).await;
            Ok(None)
        }
        Err(e) => Err(ErrorResponse::from(e)),
    }
}

/// The title a video was uploaded with, or `Ok(None)` to keep the one already shown. Only an invalid
/// video ID is a real error.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_original_title(video_id: String) -> Result<Option<String>, ErrorResponse> {
    validate_video_id(&video_id).map_err(ErrorResponse::from)?;
    Ok(crate::api::original_title::original_title(video_id.trim()).await)
}

/// RYD is a graceful-degradation integration: fetch failures resolve to
/// `Ok(None)` so the frontend simply omits the dislike counter. Only an
/// invalid video ID is a real error.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_return_youtube_dislike(
    video_id: String,
) -> Result<Option<crate::api::ryd::RydVotes>, ErrorResponse> {
    validate_video_id(&video_id).map_err(ErrorResponse::from)?;
    Ok(crate::api::ryd::fetch_ryd_votes(&video_id).await)
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_subscription_rotation_feed(
    youtube_service: State<'_, YoutubeService>,
    recommendation_service: State<'_, RecommendationService>,
) -> Result<Vec<VideoSummary>, ErrorResponse> {
    recommendation_service
        .get_subscription_rotation_feed(&youtube_service)
        .await
        .map_err(ErrorResponse::from)
}

/// Feed duration fills run beside whatever the user is playing, so they stay
/// few at a time on a slow link.
const DURATION_FILL_CONCURRENCY: usize = 3;

/// Fills in lengths, and marks the Shorts that a lookup told apart by their
/// frame, remembering each one it checked.
pub(crate) async fn populate_video_durations(
    videos: &mut [VideoSummary],
    youtube_service: &YoutubeService,
    pool: &sqlx::SqlitePool,
) {
    let items: Vec<(String, String, String, Option<String>)> = videos
        .iter()
        .map(|video| {
            (
                video.id.clone(),
                video.title.clone(),
                video.channel_name.clone(),
                video.thumbnail_url.clone(),
            )
        })
        .collect();

    let results = crate::api::http::bounded_join(
        items,
        DURATION_FILL_CONCURRENCY,
        move |(video_id, title, channel_name, thumbnail_url)| {
            let youtube_service = youtube_service.clone();
            let pool = pool.clone();
            async move {
                // 1. Try cache first
                if let Ok(Some(cached)) =
                    crate::db::cache::get_cached_video_summary(&pool, &video_id).await
                {
                    if cached.duration_seconds.is_some() {
                        return (video_id, cached.duration_seconds, None);
                    }
                }

                // 2. Fetch from API
                if let Ok(details) = youtube_service.get_video_basics(&video_id).await {
                    let summary = VideoSummary {
                        id: video_id.clone(),
                        title,
                        channel_name,
                        channel_id: details.channel_id,
                        thumbnail_url,
                        duration_seconds: details.duration_seconds,
                        published_text: None,
                        view_count_text: None,
                        channel_avatar_url: None,
                        is_live: false,
                        ..Default::default()
                    };
                    // Cache it for 7 days
                    let _ = crate::db::cache::cache_video_summary(&pool, &summary, 604800).await;
                    (video_id, details.duration_seconds, details.is_short)
                } else {
                    (video_id, None, None)
                }
            }
        },
    )
    .await;
    let mut durations = std::collections::HashMap::new();
    let mut kinds = Vec::new();
    for (id, dur, is_short) in results {
        if let Some(is_short) = is_short {
            kinds.push((id.clone(), is_short));
        }
        if let Some(d) = dur {
            durations.insert(id, d);
        }
    }

    for video in videos.iter_mut() {
        if video.duration_seconds.is_none() {
            if let Some(dur) = durations.get(&video.id) {
                video.duration_seconds = Some(*dur);
            }
        }
    }
    let shorts: std::collections::HashSet<&str> = kinds
        .iter()
        .filter(|(_, is_short)| *is_short)
        .map(|(id, _)| id.as_str())
        .collect();
    for video in videos.iter_mut() {
        video.is_short |= shorts.contains(video.id.as_str());
    }
    let _ =
        crate::db::video_kind_checks::record(pool, &kinds, chrono::Utc::now().timestamp_millis())
            .await;
}

fn dedupe_channel_ids(channel_ids: Vec<String>) -> Result<Vec<String>, ErrorResponse> {
    let mut unique = Vec::new();
    for channel_id in channel_ids {
        let trimmed = channel_id.trim().trim_start_matches("channel:").to_string();
        validate_channel_id(&trimmed).map_err(ErrorResponse::from)?;
        if !unique.contains(&trimmed) {
            unique.push(trimmed);
        }
    }
    Ok(unique)
}

const SUBSCRIPTION_RSS_BATCH_SIZE: usize = 50;

/// Videos that were refused for good when they were opened, for Subscriptions to hide.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_unplayable_video_ids(
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<Vec<String>, ErrorResponse> {
    let ids = crate::db::unplayable_videos::all(&pool)
        .await
        .map_err(ErrorResponse::from)?;
    Ok(ids.into_iter().collect())
}

/// Collaborations of the followed channels: a few more are looked at, and all known are returned.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn scan_subscription_collaborations(
    channel_ids: Vec<String>,
    youtube_service: State<'_, YoutubeService>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<Vec<VideoSummary>, ErrorResponse> {
    let unique_channel_ids = dedupe_channel_ids(channel_ids)?;
    let now_ms = u64::try_from(chrono::Utc::now().timestamp_millis()).unwrap_or(0);
    Ok(
        subscription_feed::collabs::scan(&youtube_service, &pool, &unique_channel_ids, now_ms)
            .await,
    )
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn get_subscription_rss_feed(
    channel_ids: Vec<String>,
    limit: Option<usize>,
    youtube_service: State<'_, YoutubeService>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<SubscriptionRssFeed, ErrorResponse> {
    let unique_channel_ids = dedupe_channel_ids(channel_ids)?;
    let limit = limit.unwrap_or(1500).clamp(1, 1500);
    let mut uploads =
        subscription_feed::fetch_channels(&youtube_service, &unique_channel_ids).await;
    subscription_feed::sort_and_dedupe(&mut uploads.videos);
    let videos = subscription_feed::finish_feed(
        &youtube_service,
        &pool,
        uploads.videos,
        &uploads.unclassified,
        limit,
    )
    .await;
    let total = unique_channel_ids.len();
    Ok(SubscriptionRssFeed {
        videos,
        channels: uploads.channels,
        failed_channel_ids: uploads.failed_channel_ids,
        processed: total,
        total,
    })
}

/// The feed in batches of channels, each sent as it lands, then the finished
/// feed with Shorts and lengths filled in.
#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn stream_subscription_rss_feed(
    channel_ids: Vec<String>,
    limit: Option<usize>,
    on_event: tauri::ipc::Channel<SubscriptionRssFeed>,
    youtube_service: State<'_, YoutubeService>,
    pool: State<'_, sqlx::SqlitePool>,
) -> Result<(), ErrorResponse> {
    let unique_channel_ids = dedupe_channel_ids(channel_ids)?;
    let limit = limit.unwrap_or(1500).clamp(1, 1500);
    let total = unique_channel_ids.len();
    let mut uploads = subscription_feed::ChannelUploads::default();
    let mut processed = 0usize;

    for batch in unique_channel_ids.chunks(SUBSCRIPTION_RSS_BATCH_SIZE) {
        let found = subscription_feed::fetch_channels(&youtube_service, batch).await;
        uploads.channels.extend(found.channels);
        uploads.videos.extend(found.videos);
        uploads.failed_channel_ids.extend(found.failed_channel_ids);
        uploads.unclassified.extend(found.unclassified);
        processed = (processed + batch.len()).min(total);
        subscription_feed::sort_and_dedupe(&mut uploads.videos);
        let _ = on_event.send(SubscriptionRssFeed {
            videos: subscription_feed::partial_feed(&uploads.videos, limit),
            channels: uploads.channels.clone(),
            failed_channel_ids: uploads.failed_channel_ids.clone(),
            processed,
            total,
        });
    }

    let videos = subscription_feed::finish_feed(
        &youtube_service,
        &pool,
        uploads.videos,
        &uploads.unclassified,
        limit,
    )
    .await;
    let _ = on_event.send(SubscriptionRssFeed {
        videos,
        channels: uploads.channels,
        failed_channel_ids: uploads.failed_channel_ids,
        processed: total,
        total,
    });

    Ok(())
}

/// YouTube answers a rate-limited caption request with an HTML page, which must not pass for a
/// caption file: it would parse to no lines at all and the track would just stay blank.
fn caption_status(status: reqwest::StatusCode) -> Result<(), crate::errors::AppError> {
    if status.is_success() {
        return Ok(());
    }
    Err(crate::errors::AppError::Extractor(format!(
        "Caption request refused: {status}"
    )))
}

#[cfg(not(flow_web_headless))]
#[tauri::command]
pub async fn fetch_subtitles(
    url: String,
    streaming_manager: State<'_, StreamingManager>,
) -> Result<String, ErrorResponse> {
    debug!("[fetch_subtitles] Requested fetch for: {}", url);

    if let Ok(parsed_url) = reqwest::Url::parse(&url) {
        let path = parsed_url.path();
        if let Some(token) = path.strip_prefix("/stream/") {
            let token = token.trim_start_matches('/');
            debug!(
                "[fetch_subtitles] Local proxy URL detected. Extracted token: {}",
                token
            );
            if let Some(session) = streaming_manager.get_session(token) {
                debug!(
                    "[fetch_subtitles] Found stream session for token: {}",
                    token
                );
                match session.kind {
                    crate::streaming::proxy::StreamSessionKind::Remote { remote_url } => {
                        debug!("[fetch_subtitles] Remote URL session found: {}", remote_url);
                        let user_agent = if session.user_agent.is_empty() {
                            crate::api::http::BROWSER_USER_AGENT
                        } else {
                            debug!(
                                "[fetch_subtitles] Using session User-Agent: {}",
                                session.user_agent
                            );
                            session.user_agent.as_str()
                        };
                        let client = crate::api::http::shared_client();

                        let res = client
                            .get(&remote_url)
                            .header(reqwest::header::USER_AGENT, user_agent)
                            .send()
                            .await
                            .map_err(|e| {
                                crate::errors::AppError::Extractor(format!(
                                    "Network error fetching subtitles: {}",
                                    e
                                ))
                            })?;

                        caption_status(res.status())?;
                        let text = res.text().await.map_err(|e| {
                            crate::errors::AppError::Extractor(format!(
                                "Read error fetching subtitles: {}",
                                e
                            ))
                        })?;

                        debug!(
                            "[fetch_subtitles] Successfully fetched remote subtitles. Length: {} bytes",
                            text.len()
                        );
                        return Ok(text);
                    }
                    crate::streaming::proxy::StreamSessionKind::Inline { body } => {
                        debug!(
                            "[fetch_subtitles] Inline session found with {} bytes.",
                            body.len()
                        );
                        let text = String::from_utf8(body).map_err(|e| {
                            crate::errors::AppError::Extractor(format!(
                                "UTF-8 decoding error: {}",
                                e
                            ))
                        })?;
                        return Ok(text);
                    }
                    crate::streaming::proxy::StreamSessionKind::Local { path } => {
                        let text = tokio::fs::read_to_string(&path).await.map_err(|e| {
                            crate::errors::AppError::Extractor(format!(
                                "Read error reading local subtitles: {}",
                                e
                            ))
                        })?;
                        return Ok(text);
                    }
                }
            } else {
                debug!(
                    "[fetch_subtitles] No active/expired session found in StreamingManager for token: {}",
                    token
                );
            }
        }
    }

    debug!(
        "[fetch_subtitles] Falling back to direct URL fetch: {}",
        url
    );
    let client = crate::api::http::shared_client();

    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| crate::errors::AppError::Extractor(format!("Network error: {}", e)))?;
    caption_status(res.status())?;

    let text = res
        .text()
        .await
        .map_err(|e| crate::errors::AppError::Extractor(format!("Read error: {}", e)))?;

    debug!(
        "[fetch_subtitles] Direct fetch successful. Length: {} bytes",
        text.len()
    );
    Ok(text)
}

#[cfg(test)]
mod caption_status_tests {
    use super::caption_status;
    use reqwest::StatusCode;

    #[test]
    fn a_rate_limited_answer_is_a_failure_not_a_caption_file() {
        assert!(caption_status(StatusCode::OK).is_ok());
        assert!(caption_status(StatusCode::TOO_MANY_REQUESTS).is_err());
        assert!(caption_status(StatusCode::FORBIDDEN).is_err());
    }
}

#[cfg(test)]
mod non_music_segment_tests {
    use super::{NonMusicSegment, non_music_segments_url, parse_hashed_segments};

    #[test]
    fn the_lookup_names_only_a_hash_prefix_of_the_video() {
        let url = non_music_segments_url("https://sponsor.ajay.app", "dQw4w9WgXcQ");
        assert!(!url.contains("dQw4w9WgXcQ"));
        let prefix = url
            .strip_prefix("https://sponsor.ajay.app/api/skipSegments/")
            .and_then(|rest| rest.split('?').next())
            .unwrap();
        assert_eq!(prefix.len(), 4);
        assert!(url.contains("music_offtopic"));
    }

    #[test]
    fn only_the_playing_videos_skippable_non_music_parts_are_kept() {
        let json = r#"[
            {"videoID":"other","segments":[{"category":"music_offtopic","actionType":"skip","segment":[1,2],"videoDuration":10}]},
            {"videoID":"wanted","segments":[
                {"category":"music_offtopic","actionType":"skip","segment":[0,30.5],"videoDuration":240},
                {"category":"sponsor","actionType":"skip","segment":[40,50],"videoDuration":240},
                {"category":"music_offtopic","actionType":"mute","segment":[60,70],"videoDuration":240},
                {"category":"music_offtopic","actionType":"skip","segment":[80]}
            ]}
        ]"#;
        assert_eq!(
            parse_hashed_segments(json, "wanted").unwrap(),
            vec![NonMusicSegment {
                start_seconds: 0.0,
                end_seconds: 30.5,
                video_duration_seconds: 240.0
            }]
        );
        assert!(parse_hashed_segments(json, "missing").unwrap().is_empty());
        assert!(parse_hashed_segments("not json", "wanted").is_err());
    }
}

fn allowed_remote(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https" && url.port().is_none() && url.username().is_empty() && url.password().is_none()
        && url.host_str().is_some_and(|host| ["googlevideo.com", "youtube.com", "youtube-nocookie.com"].iter().any(|domain| host == *domain || host.ends_with(&format!(".{domain}"))))
    })
}
pub(crate) fn register(manager: &StreamingManager, id: &str, url: &str, mime: &str, agent: &str) -> String {
    if !allowed_remote(url) { return String::new(); }
    let token = uuid::Uuid::new_v4().to_string();
    manager.register_video_stream(id, token.clone(), url.into(), mime.into(), agent.into());
    format!("/media/stream/{token}")
}
pub fn prepare(manager: &StreamingManager, id: &str, refresh: bool, mut info: StreamInfo) -> AppResult<StreamInfo> {
    let agent = stream_user_agent(&info);
    info.expires_at = info.expires_at.split('|').next().unwrap_or_default().to_string();
    info.local_url = register(manager, id, &info.local_url, "video/mp4", &agent);
    for variant in &mut info.variants {
        let mime = variant.mime_type.as_deref().and_then(|m|m.split(';').next()).unwrap_or("video/mp4");
        variant.local_url = register(manager, id, &variant.local_url, mime, &agent);
        variant.is_playable &= !variant.local_url.is_empty();
    }
    for track in &mut info.audio_tracks {
        let mime = track.mime_type.as_deref().and_then(|m|m.split(';').next()).unwrap_or("audio/mp4");
        track.local_url = register(manager, id, &track.local_url, mime, track.user_agent.as_deref().unwrap_or(&agent));
        track.available &= !track.local_url.is_empty();
    }
    for caption in &mut info.captions {
        caption.url = register(manager, id, &caption.url, "text/vtt; charset=utf-8", crate::api::http::BROWSER_USER_AGENT);
    }
    info.captions.retain(|caption| !caption.url.is_empty());

    // Upstream manifest URL overrides are designed for trusted desktop IPC,
    // not a network API. Expose only generated DASH manifests and SABR sessions.
    info.hls_manifest_url = None;
    info.dash_manifest_url = None;
    if let Some(xml) = crate::streaming::dash_manifest::build_synthetic_dash_manifest(&info) {
        let token = uuid::Uuid::new_v4().to_string();
        manager.register_inline_session(token.clone(), xml.into_bytes(), "application/dash+xml".into());
        info.dash_manifest_url = Some(format!("/media/stream/{token}"));
    }
    if let Some(descriptor) = info.sabr_descriptor.take().filter(|d| allowed_remote(&d.server_abr_streaming_url)) {
        if refresh { manager.sabr().discard_video(&descriptor.video_id); }
        let session = manager.sabr().prepare(descriptor, crate::streaming::sabr::CodecSupport::default());
        if let Some(sabr) = info.sabr.as_mut() {
            sabr.manifest_url = Some(format!("/media/sabr/{session}/manifest.mpd"));
        }
    } else {
        info.sabr = None;
    }
    if info.local_url.is_empty() && info.dash_manifest_url.is_none() && info.sabr.is_none()
        && !info.variants.iter().any(|v| v.is_playable && v.has_audio) {
        return Err(AppError::Streaming("No browser-compatible stream was found in this experimental build".into()));
    }
    Ok(info)
}
pub async fn stream(State(state): State<AppState>, Path(token): Path<String>, method: Method, headers: HeaderMap) -> Response {
    if uuid::Uuid::parse_str(&token).is_err() || state.streams.get_session(&token).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    relay(&state, &format!("/stream/{token}"), method, headers, false).await
}
fn valid_sabr(path: &str) -> bool {
    if path.len() > 256 || !path.bytes().all(|c| c.is_ascii_alphanumeric() || b"/_.-".contains(&c)) || path.contains("..") { return false; }
    let segments: Vec<_> = path.split('/').collect();
    matches!(segments.as_slice(), [_, "manifest.mpd"] | [_, "video", "init"] | [_, "video", "seg", _] | [_, "audio", _, "init"] | [_, "audio", _, "seg", _])
}
pub async fn sabr(State(state): State<AppState>, Path(path): Path<String>, method: Method, headers: HeaderMap) -> Response {
    if !valid_sabr(&path) { return StatusCode::NOT_FOUND.into_response(); }
    relay(&state, &format!("/sabr/{path}"), method, headers, path.ends_with("/manifest.mpd")).await
}
async fn relay(state: &AppState, path: &str, method: Method, headers: HeaderMap, rewrite_manifest: bool) -> Response {
    let origin = format!("http://127.0.0.1:{}", state.streams.get_port());
    let mut request = state.relay.request(method.clone(), format!("{origin}{path}"));
    if let Some(range) = headers.get(header::RANGE) {
        if !rewrite_manifest { request = request.header(header::RANGE, range); }
    }
    let Ok(upstream) = request.send().await else { return StatusCode::BAD_GATEWAY.into_response(); };
    let status = upstream.status();
    let mut response = Response::builder().status(status);
    for name in [header::CONTENT_TYPE, header::CONTENT_RANGE, header::ACCEPT_RANGES, header::CONTENT_ENCODING] {
        if let Some(value) = upstream.headers().get(&name) { response = response.header(name, value); }
    }
    if rewrite_manifest && status.is_success() && method != Method::HEAD {
        let Ok(xml) = upstream.text().await else { return StatusCode::BAD_GATEWAY.into_response(); };
        let xml = xml.replace(&format!("{origin}/sabr/"), "/media/sabr/");
        return response.body(Body::from(xml)).unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response());
    }
    if let Some(length) = upstream.headers().get(header::CONTENT_LENGTH) {
        response = response.header(header::CONTENT_LENGTH, length);
    }
    response.body(Body::from_stream(upstream.bytes_stream())).unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_arbitrary_proxy_targets() {
        assert!(allowed_remote("https://rr1.googlevideo.com/videoplayback"));
        for url in ["http://127.0.0.1", "https://googlevideo.com.evil.test/", "https://user@youtube.com/", "https://youtube.com:1234/"] {
            assert!(!allowed_remote(url));
        }
    }
    #[test]
    fn sabr_paths_are_constrained() {
        assert!(valid_sabr("abc/video/seg/1"));
        assert!(!valid_sabr("abc/../manifest.mpd"));
        assert!(!valid_sabr("abc/proxy/http://localhost"));
    }
}
