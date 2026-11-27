// The original SponsorBlock readers, restricted to the public service in web mode.
use crate::validation::validate_video_id;
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
pub(crate) async fn fetch_non_music_segments(
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

fn normalize_sponsorblock_server_url(_server_url: Option<&str>) -> crate::errors::AppResult<String> {
    Ok("https://sponsor.ajay.app".into())
}

const SPONSORBLOCK_USER_AGENT: &str = "FlowWeb/1.0";
const SPONSORBLOCK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

