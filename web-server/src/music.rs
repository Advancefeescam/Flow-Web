use crate::{AppState, api, errors::{AppError, AppResult}, music_service::Freshness, rpc::{arg,token,video}, validation::{validate_search_query,validate_browse_id,validate_video_id}};
use serde_json::{Value,json};

fn required_token(args: &Value, key: &str) -> AppResult<String> {
    token(args,key)?.ok_or_else(|| AppError::Validation(format!("Missing {key}")))
}
fn browse(args: &Value, key: &str) -> AppResult<String> {
    let id: String = arg(args,key)?; validate_browse_id(&id)?; Ok(id)
}
fn cache_stamp() -> String {
    let context = api::innertube::music::clients::WEB_REMIX.context(None,None);
    let c = &context["client"];
    format!("{}-{}",c["hl"].as_str().unwrap_or_default(),c["gl"].as_str().unwrap_or_default())
}
pub async fn dispatch(state: &AppState, command: &str, args: &Value) -> AppResult<Option<Value>> {
    let m = &state.music;
    macro_rules! out { ($expr:expr) => { serde_json::to_value($expr.await?)? }; }
    let value = match command {
        "get_music_home_page" => {
            let continuation = token(args,"continuation")?;
            let page = m.home(continuation.as_deref()).await?;
            if continuation.is_none() { let _ = crate::db::music_home::save(&state.pool,&cache_stamp(),&page).await; }
            json!(page)
        },
        "get_cached_music_home_page" => out!(crate::db::music_home::get_cached(&state.pool,&cache_stamp())),
        "get_music_request_diagnostics" => json!(api::innertube::music::client::music_request_diagnostics()),
        "get_music_explore_page" => out!(m.explore()),
        "get_music_charts_page" => {
            let country: Option<String> = arg(args,"country")?;
            out!(m.charts(token(args,"continuation")?.as_deref(),country.as_deref()))
        },
        "get_music_trending_songs" => {
            let country: Option<String> = arg(args,"country")?;
            out!(m.trending_songs(country.as_deref()))
        },
        "get_music_moods" => out!(m.moods()),
        "get_music_mood_groups" => out!(m.mood_groups()),
        "get_music_new_releases" => out!(m.new_releases()),
        "get_music_mood_genre" | "get_music_artist_items" => out!(m.mood_genre(&browse(args,"browseId")?,token(args,"params")?.as_deref(),token(args,"continuation")?.as_deref())),
        "search_music_typed" => {
            let query: String = arg(args,"query")?; validate_search_query(&query)?;
            let filter: String = arg(args,"filter")?;
            out!(m.search(&query,&filter))
        },
        "search_music_continuation" => out!(m.search_continuation(&required_token(args,"continuation")?)),
        "get_music_search_summary" | "get_music_search_suggestions" => {
            let query: String = arg(args,"query")?; validate_search_query(&query)?;
            if command == "get_music_search_summary" { out!(m.search_summary(&query)) } else { out!(m.search_suggestions(&query)) }
        },
        "get_music_album_page" => out!(m.album(&browse(args,"browseId")?,Freshness::prefer_cached(arg(args,"preferCached")?))),
        "get_music_album_continuation" => out!(m.album_continuation(&required_token(args,"continuation")?)),
        "get_music_artist_page" => out!(m.artist(&browse(args,"browseId")?,Freshness::prefer_cached(arg(args,"preferCached")?))),
        "get_music_playlist_page" => out!(m.playlist(&browse(args,"playlistId")?,Freshness::prefer_cached(arg(args,"preferCached")?))),
        "get_music_playlist_continuation" => out!(m.playlist_continuation(&required_token(args,"continuation")?)),
        "get_music_watch_queue" => {
            let id: Option<String> = arg(args,"videoId")?;
            if let Some(ref id) = id { validate_video_id(id)?; }
            let playlist: Option<String> = arg(args,"playlistId")?;
            if let Some(ref p) = playlist { validate_browse_id(p)?; }
            out!(m.watch_queue(id.as_deref(),playlist.as_deref(),token(args,"params")?.as_deref()))
        },
        "get_music_queue_continuation" => out!(m.queue_continuation(&required_token(args,"continuation")?)),
        "get_music_queue" => {
            let ids: Vec<String> = arg(args,"videoIds")?;
            if ids.len()>500 { return Err(AppError::Validation("Too many tracks".into())); }
            for id in &ids { validate_video_id(id)?; }
            let playlist: Option<String> = arg(args,"playlistId")?;
            if let Some(ref p) = playlist { validate_browse_id(p)?; }
            out!(m.get_queue(&ids,playlist.as_deref()))
        },
        "get_music_related_typed" => out!(m.related(&video(args)?)),
        "get_music_lyrics_typed" => out!(m.lyrics(&video(args)?)),
        "get_music_captions_lrc" => out!(m.captions_lrc(&video(args)?)),
        "romanize_lyrics" => {
            let lines: Vec<String> = arg(args,"lines")?;
            json!(api::lyrics_romanize::romanize_lines(&lines))
        },
        "get_music_stream" => {
            let id = video(args)?;
            let quality: Option<String> = arg(args,"audioQuality")?;
            let quality = quality.as_deref().unwrap_or("Auto").parse().map_err(AppError::Validation)?;
            let prefer_aac: Option<bool> = arg(args,"preferAac")?;
            let mut info = m.resolve_stream(&id,quality,prefer_aac.unwrap_or(true)).await?;
            let mime = info.mime_type.split(';').next().unwrap_or("audio/mp4");
            info.audio_url = crate::media::register(&state.streams,&id,&info.audio_url,mime,&info.user_agent);
            if info.audio_url.is_empty() { return Err(AppError::Streaming("No supported audio URL".into())); }
            info.user_agent.clear();
            json!(info)
        },
        _ => return Ok(None),
    };
    Ok(Some(value))
}
