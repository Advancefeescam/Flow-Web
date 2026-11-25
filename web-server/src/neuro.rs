use crate::{
    AppState,
    errors::{AppError, AppResult},
    flow_neuro::signals::InteractionType,
    models::video::VideoSummary,
    rpc::{arg, video},
    validation::validate_video_id,
};
use serde_json::{Value, json};

fn interaction(value: &str) -> AppResult<InteractionType> {
    match value.to_ascii_uppercase().as_str() {
        "CLICK" => Ok(InteractionType::Click),
        "LIKED" => Ok(InteractionType::Liked),
        "WATCHED" => Ok(InteractionType::Watched),
        "SKIPPED" => Ok(InteractionType::Skipped),
        "DISLIKED" => Ok(InteractionType::Disliked),
        "SAVED" => Ok(InteractionType::Saved),
        _ => Err(AppError::Validation("Invalid interaction type".into())),
    }
}

pub async fn dispatch(state: &AppState, command: &str, args: &Value) -> AppResult<Option<Value>> {
    let service = &state.recommendations;
    let value = match command {
        "log_interaction" | "mark_not_interested" => {
            let id = video(args)?;
            let title: String = arg(args, "title")?;
            let channel_name: String = arg(args, "channelName")?;
            let channel_id: String = arg(args, "channelId")?;
            let description: Option<String> = arg(args, "description")?;
            let duration: Option<u64> = arg(args, "durationSeconds")?;
            let is_live: Option<bool> = arg(args, "isLive")?;
            let is_short: Option<bool> = arg(args, "isShort")?;
            let (kind, percent) = if command == "mark_not_interested" {
                (InteractionType::NotInterested, 0.0)
            } else {
                let kind: String = arg(args, "interactionType")?;
                let percent: Option<f32> = arg(args, "percentWatched")?;
                (interaction(&kind)?, percent.unwrap_or(0.0))
            };
            service.log_video_interaction(
                &id,
                &title,
                &channel_name,
                &channel_id,
                description.as_deref(),
                duration,
                is_live.unwrap_or(false),
                is_short.unwrap_or(false),
                kind,
                percent.clamp(0.0, 1.0),
                None,
            ).await?;
            Value::Null
        }
        "report_watch_progress" => {
            let report = arg(args, "report")?;
            service.report_watch_progress(report).await?;
            Value::Null
        }
        "end_watch_session" => {
            let report = arg(args, "report")?;
            let natural: bool = arg(args, "natural")?;
            if let Some(generation) = service.end_watch_session(report, natural).await? {
                let service = service.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                    if let Err(error) = service.settle_watch_session(Some(generation)).await {
                        tracing::warn!(%error, "FlowNeuro watch session settlement failed");
                    }
                });
            }
            Value::Null
        }
        "record_feed_impressions" => {
            let videos: Vec<VideoSummary> = arg(args, "videos")?;
            if videos.len() > 2_000 { return Err(AppError::Validation("Too many feed impressions".into())); }
            service.record_feed_impressions(videos).await?;
            Value::Null
        }
        "record_seen_videos" => {
            let ids: Vec<String> = arg(args, "ids")?;
            if ids.len() > 10_000 { return Err(AppError::Validation("Too many video IDs".into())); }
            for id in &ids { validate_video_id(id)?; }
            crate::db::seen_videos::remember(&state.pool, &ids, chrono::Utc::now().timestamp_millis()).await?;
            Value::Null
        }
        "seen_video_ids_among" => {
            let ids: Vec<String> = arg(args, "ids")?;
            if ids.len() > 10_000 { return Err(AppError::Validation("Too many video IDs".into())); }
            for id in &ids { validate_video_id(id)?; }
            json!(crate::db::seen_videos::among(&state.pool, &ids).await?)
        }
        "complete_onboarding" => {
            let topics: Vec<String> = arg(args, "topics")?;
            if topics.len() > 100 { return Err(AppError::Validation("Too many onboarding topics".into())); }
            service.complete_onboarding(topics).await?;
            json!(null)
        }
        "get_onboarding_status" => json!(service.get_onboarding_status().await?),
        "get_flow_persona" => {
            let persona = service.get_personality().await?;
            json!({"name":persona.name(),"title":persona.title(),"description":persona.description(),"icon":persona.icon()})
        }
        "get_brain_snapshot" => json!(service.get_brain_snapshot().await?),
        "get_taste_overview" => json!(service.taste_overview().await),
        "get_neighbour_channels" => {
            let subscriptions = crate::home_feed::inputs::subscriptions(&state.pool).await?;
            let stored = crate::home_feed::neighbours::load(&state.pool).await?;
            let found = service.neighbour_channels(&subscriptions.names).await;
            json!(crate::home_feed::neighbours::discoverable(found, &stored))
        }
        "forget_remembered_channel" => {
            let id: String = arg(args, "channelId")?;
            validate_channel_id(&id)?;
            let now = u64::try_from(chrono::Utc::now().timestamp_millis()).unwrap_or(0);
            crate::home_feed::channel_memory::forget(&state.pool, &id, now).await?;
            Value::Null
        }
        "clear_channel_memory" => {
            let now = u64::try_from(chrono::Utc::now().timestamp_millis()).unwrap_or(0);
            crate::home_feed::channel_memory::clear(&state.pool, now).await?;
            Value::Null
        }
        "resolve_hidden_channel_names" => {
            let ids: Vec<String> = arg(args, "channelIds")?;
            if ids.len() > 500 { return Err(AppError::Validation("Too many channel IDs".into())); }
            for id in &ids { validate_channel_id(id)?; }
            let blocked: std::collections::HashSet<String> = service.get_brain_snapshot().await?.blocked_channels.into_iter().collect();
            json!(state.blocked_channel_names.resolve(&state.pool, ids, &blocked).await)
        }
        "get_recommendation_events" => {
            let limit: Option<i64> = arg(args, "limit")?;
            json!(service.get_recommendation_log(limit.unwrap_or(50).clamp(1, 500)).await?)
        }
        "learn_from_search" => {
            let query: String = arg(args, "query")?;
            if query.len() > 500 { return Err(AppError::Validation("Search query is too long".into())); }
            json!(service.learn_from_search(query).await?)
        }
        "note_subscription" => {
            let id: String = arg(args, "channelId")?;
            let name: String = arg(args, "channelName")?;
            let subscribed: bool = arg(args, "subscribed")?;
            service.note_subscription(id, name, subscribed).await?;
            Value::Null
        }
        "observe_channel_uploads" => {
            let uploads: Vec<VideoSummary> = arg(args, "uploads")?;
            if uploads.len() > 500 { return Err(AppError::Validation("Too many channel uploads".into())); }
            service.observe_channel_uploads(uploads).await?;
            Value::Null
        }
        "bootstrap_from_history" => {
            let videos = arg(args, "videos")?;
            json!(service.bootstrap_from_history(videos).await?)
        }
        "bootstrap_from_subscriptions" => {
            let names = arg(args, "channelNames")?;
            json!(service.bootstrap_from_subscriptions(names).await?)
        }
        "import_brain" => {
            let brain = arg(args, "brain")?;
            service.import_brain(brain).await?;
            Value::Null
        }
        "reset_brain" => { service.reset_brain().await?; Value::Null }
        "unblock_topic" => {
            let topic = arg(args, "topic")?;
            service.unblock_topic(topic).await?;
            Value::Null
        }
        "add_blocked_topic" => {
            let topic = arg(args, "topic")?;
            service.add_blocked_topic(topic).await?;
            Value::Null
        }
        "add_preferred_topic" => {
            let topic = arg(args, "topic")?;
            service.add_preferred_topic(topic).await?;
            Value::Null
        }
        "remove_preferred_topic" => {
            let topic = arg(args, "topic")?;
            service.remove_preferred_topic(topic).await?;
            Value::Null
        }
        "unblock_channel" => {
            let id: String = arg(args, "channelId")?;
            service.unblock_channel(id).await?;
            Value::Null
        }
        "block_channel" => {
            let id = arg(args, "channelId")?;
            let name: Option<String> = arg(args, "channelName")?;
            service.block_channel(id.clone()).await?;
            if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
                crate::db::settings::set_setting(&state.pool, &format!("blocked_channel_name:{id}"), &name).await?;
            }
            Value::Null
        }
        _ => return Ok(None),
    };
    Ok(Some(value))
}
