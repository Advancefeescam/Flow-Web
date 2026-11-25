//! Flow's shared personalized Shorts engine, exposed through browser RPC.
use serde_json::Value;
use std::collections::HashSet;

use crate::{
    AppState,
    errors::{AppError, AppResult},
    services::shorts_engine::ShortView,
    validation::{validate_channel_id, validate_page_token, validate_video_id},
};

fn ids(args: &Value) -> AppResult<Vec<String>> {
    let ids: Vec<String> = crate::rpc::arg(args, "userSubs")?;
    if ids.len() > 2_000 { return Err(AppError::Validation("Too many subscription IDs".into())); }
    for id in &ids { validate_channel_id(id)?; }
    Ok(ids)
}

fn region(args: &Value) -> AppResult<Option<String>> {
    let region: Option<String> = crate::rpc::arg(args, "region")?;
    if region.as_ref().is_some_and(|r| r.len() > 8 || !r.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')) {
        return Err(AppError::Validation("Invalid region".into()));
    }
    Ok(region)
}

async fn remember(state: &AppState, feed: &crate::models::shorts::ShortsFeed) {
    let ids: Vec<String> = feed.items.iter().map(|item| item.id.clone()).collect();
    if let Err(error) = crate::db::known_shorts::remember(
        &state.pool, &ids, chrono::Utc::now().timestamp_millis(),
    ).await {
        tracing::warn!(%error, "Could not store Shorts IDs");
    }
}

pub async fn dispatch(state: &AppState, command: &str, args: &Value) -> AppResult<Option<Value>> {
    let value = match command {
        "get_shorts_feed" => {
            let seed: Option<String> = crate::rpc::arg(args, "seedId")?;
            if let Some(id) = &seed { validate_video_id(id)?; }
            let user_subs = ids(args)?;
            let region = region(args)?;
            let feed = state.shorts.get_feed(
                &state.youtube, &state.recommendations, seed, user_subs, region,
            ).await?;
            remember(state, &feed).await;
            serde_json::to_value(feed)?
        }
        "load_more_shorts" => {
            let continuation: Option<String> = crate::rpc::arg(args, "continuation")?;
            if let Some(value) = &continuation { validate_page_token(value)?; }
            let user_subs = ids(args)?;
            let region = region(args)?;
            let feed = state.shorts.load_more(
                &state.youtube, &state.recommendations, continuation, user_subs, region,
            ).await?;
            remember(state, &feed).await;
            serde_json::to_value(feed)?
        }
        "reset_shorts_feed" => { state.shorts.reset(); Value::Null }
        "report_short_view" => {
            let view: ShortView = crate::rpc::arg(args, "view")?;
            validate_video_id(&view.video_id)?;
            state.recommendations.report_short_view(view).await?;
            Value::Null
        }
        "record_seen_shorts" => {
            let ids: Vec<String> = crate::rpc::arg(args, "ids")?;
            if ids.len() > 2_000 { return Err(AppError::Validation("Too many Short IDs".into())); }
            for id in &ids { validate_video_id(id)?; }
            state.recommendations.record_seen_shorts(&ids).await;
            Value::Null
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
