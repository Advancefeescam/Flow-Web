use axum::{Json, extract::{Path, State}, http::StatusCode, response::{IntoResponse, Response}};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use crate::{AppState, errors::{AppError, AppResult, ErrorResponse}, models::{explore::ExploreTarget, search::SearchVideosRequest}, validation::*};

fn arg<T: DeserializeOwned>(args: &Value, key: &str) -> AppResult<T> {
    serde_json::from_value(args.get(key).cloned().unwrap_or(Value::Null))
        .map_err(|_| AppError::Validation(format!("Invalid or missing argument: {key}")))
}

fn video(args: &Value) -> AppResult<String> {
    let id: String = arg(args, "videoId")?;
    validate_video_id(&id)?;
    Ok(id)
}
fn channel(args: &Value) -> AppResult<String> {
    let id: String = arg(args, "channelId")?;
    validate_channel_id(&id)?;
    Ok(id)
}
fn token(args: &Value, key: &str) -> AppResult<Option<String>> {
    let value: Option<String> = arg(args, key)?;
    if let Some(ref value) = value { validate_page_token(value)?; }
    Ok(value)
}

pub async fn handle(State(state): State<AppState>, Path(command): Path<String>, Json(args): Json<Value>) -> Response {
    if !args.is_object() { return error(AppError::Validation("Expected a JSON object".into())); }
    let Ok(_permit) = state.requests.try_acquire() else {
        return error(AppError::RateLimited("Flow Web is busy. Try again.".into()));
    };
    match tokio::time::timeout(std::time::Duration::from_secs(85), dispatch(&state, &command, &args)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(err)) => error(err),
        Err(_) => error(AppError::Network("The request timed out".into())),
    }
}

fn error(err: AppError) -> Response {
    let code = match &err {
        AppError::Validation(_) => StatusCode::BAD_REQUEST,
        AppError::RateLimited(_) => StatusCode::TOO_MANY_REQUESTS,
        AppError::Network(_) => StatusCode::BAD_GATEWAY,
        AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    };
    (code, Json(ErrorResponse::from(err))).into_response()
}

fn unsupported(command: &str) -> AppError {
    AppError::Validation(format!("Command {command} is not available in this experimental web build"))
}

async fn dispatch(state: &AppState, command: &str, args: &Value) -> AppResult<Value> {
    let y = &state.youtube;
    macro_rules! output { ($expr:expr) => { Ok(serde_json::to_value($expr.await?)?) }; }
    match command {
        "search_videos" => {
            let request: SearchVideosRequest = arg(args, "request")?;
            if let Some(ref page) = request.page_token { validate_page_token(page)?; }
            else { validate_search_query(&request.query)?; }
            output!(y.search_videos(request))
        },
        "get_video_details" => output!(y.get_video_details(&video(args)?)),
        "get_video_metadata" => output!(y.get_video_metadata(&video(args)?)),
        "get_video_category" => output!(y.get_video_category(&video(args)?)),
        "get_paid_promotion" => output!(y.get_paid_promotion(&video(args)?)),
        "get_video_storyboard" => output!(y.get_video_storyboard(&video(args)?)),
        "get_related_videos" => output!(y.get_related_videos(&video(args)?)),
        "get_more_related_videos" => {
            let value = token(args, "continuation")?.ok_or_else(|| AppError::Validation("Continuation required".into()))?;
            output!(y.get_more_related_videos(&value))
        },
        "get_stream_info" => {
            let id = video(args)?;
            let refresh: Option<bool> = arg(args, "refresh")?;
            let info = y.get_stream_info(&id, refresh.unwrap_or(false)).await?;
            Ok(serde_json::to_value(crate::media::prepare(&state.streams, &id, refresh.unwrap_or(false), info)?)?)
        },
        "get_channel_details" => output!(y.get_channel_details(&channel(args)?)),
        "get_channel_page" => output!(y.get_channel_page(&channel(args)?)),
        "get_channel_about" => {
            let value = token(args, "token")?.ok_or_else(|| AppError::Validation("Token required".into()))?;
            output!(y.get_channel_about(&value))
        },
        "get_channel_tab" => output!(y.get_channel_tab(&channel(args)?, token(args,"params")?, token(args,"pageToken")?, arg(args,"query")?)),
        "get_playlist_details" => {
            let id: String = arg(args,"playlistId")?;
            validate_playlist_id(&id)?;
            output!(y.get_playlist_details(&id, token(args,"pageToken")?))
        },
        "get_mix_playlist" => {
            let id: String = arg(args,"playlistId")?;
            validate_playlist_id(&id)?;
            let video_id: Option<String> = arg(args,"videoId")?;
            if let Some(ref v) = video_id { validate_video_id(v)?; }
            output!(y.get_mix_playlist(&id, video_id.as_deref()))
        },
        "get_comments" => output!(y.get_comments(&video(args)?, token(args,"pageToken")?)),
        "get_short_comments" => output!(y.get_short_comments(&video(args)?, token(args,"pageToken")?)),
        "get_timed_comments" => {
            let value = token(args, "sectionToken")?.ok_or_else(|| AppError::Validation("Token required".into()))?;
            output!(y.get_timed_comments(&video(args)?, &value))
        },
        "get_post_comments" => {
            let id: String = arg(args,"postId")?;
            validate_browse_id(&id)?;
            output!(y.get_post_comments(&id, token(args,"params")?, token(args,"pageToken")?))
        },
        "get_live_chat" => output!(y.get_live_chat(&video(args)?, token(args,"continuation")?, arg(args,"replayOffsetMs")?)),
        "get_explore_destination" => {
            let destination: String = arg(args,"destination")?;
            let region: Option<String> = arg(args,"region")?;
            output!(y.get_explore_destination(&destination, region.as_deref()))
        },
        "get_explore_page" => {
            let target: ExploreTarget = arg(args,"target")?;
            validate_browse_id(&target.browse_id)?;
            let region: Option<String> = arg(args,"region")?;
            output!(y.get_explore_page(&target.browse_id, target.params.as_deref(), region.as_deref()))
        },
        "get_explore_grid" => {
            let target: Option<ExploreTarget> = arg(args,"target")?;
            if let Some(ref t) = target { validate_browse_id(&t.browse_id)?; }
            let continuation = token(args,"continuation")?;
            let region: Option<String> = arg(args,"region")?;
            output!(y.get_explore_grid(target.as_ref(), continuation.as_deref(), region.as_deref()))
        },
        "get_search_suggestions" => {
            let query: String = arg(args,"query")?;
            validate_search_query(&query)?;
            let region: Option<String> = arg(args,"region")?;
            output!(y.get_search_suggestions(&query, region.as_deref()))
        },
        "warm_search_suggestions" => { y.warm_search_suggestions().await; Ok(Value::Null) },
        "parse_subscription_export" => {
            let data: String = arg(args,"data")?;
            Ok(json!(y.parse_subscription_export(&data)?))
        },
        _ => Err(unsupported(command)),
    }
}
