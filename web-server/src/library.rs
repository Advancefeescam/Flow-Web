use crate::{AppState, api::innertube::{core::{locale, remote_decoder}, extractors::audio_preference}, db::{settings, watch_history::{self, WatchHistoryRecord}}, errors::{AppError, AppResult}, validation::validate_video_id};
use serde_json::{Value, json};

const BACKEND_KEYS: [&str; 4] = [
    remote_decoder::SETTING_KEY, audio_preference::STABLE_VOLUME_SETTING,
    locale::CONTENT_LANGUAGE_SETTING, locale::CONTENT_COUNTRY_SETTING,
];

pub async fn load_settings(pool: &sqlx::SqlitePool) -> AppResult<()> {
    for key in BACKEND_KEYS {
        apply_setting(key, settings::get_setting(pool, key).await?.as_deref());
    }
    Ok(())
}

fn apply_setting(key: &str, value: Option<&str>) {
    let enabled = value.is_some_and(|v| v.eq_ignore_ascii_case("true"));
    match key {
        remote_decoder::SETTING_KEY => remote_decoder::set_enabled(value.is_some_and(|v| v.eq_ignore_ascii_case("true"))),
        audio_preference::STABLE_VOLUME_SETTING => audio_preference::set_stable_volume(enabled),
        locale::CONTENT_LANGUAGE_SETTING => locale::set_content_language(value),
        locale::CONTENT_COUNTRY_SETTING => locale::set_content_country(value),
        _ => {}
    }
}

fn check_key(key: &str) -> AppResult<()> {
    if key.is_empty() || key.len() > 128 || !key.bytes().all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c)) {
        return Err(AppError::Validation("Invalid setting key".into()));
    }
    // Outbound proxy configuration is an operator setting, not a browser capability.
    if key.starts_with("proxy_") {
        return Err(AppError::Validation("Server network configuration is managed by the operator".into()));
    }
    Ok(())
}
fn check_record(record: &WatchHistoryRecord) -> AppResult<()> {
    validate_video_id(&record.video_id)?;
    if record.watch_duration_seconds < 0 || record.total_duration_seconds.is_some_and(|s| s < 0)
        || record.title.len() > 8192 || record.watch_date.len() > 100 {
        return Err(AppError::Validation("Invalid watch record".into()));
    }
    Ok(())
}

pub async fn dispatch(state: &AppState, command: &str, args: &Value) -> AppResult<Option<Value>> {
    let p = &state.pool;
    let arg = |key: &str| args.get(key).cloned().unwrap_or(Value::Null);
    macro_rules! read {
        ($key:literal, $ty:ty) => { serde_json::from_value::<$ty>(arg($key))
            .map_err(|_| AppError::Validation(concat!("Invalid argument: ", $key).into()))? };
    }
    let result = match command {
        "get_settings" => {
            let keys = read!("keys", Vec<String>);
            if keys.len() > 500 { return Err(AppError::Validation("Too many settings".into())); }
            let mut values = serde_json::Map::new();
            for key in keys {
                if key.starts_with("proxy_") { values.insert(key, Value::Null); continue; }
                check_key(&key)?;
                values.insert(key.clone(), json!(settings::get_setting(p, &key).await?));
            }
            Value::Object(values)
        },
        "get_setting" => {
            let key = read!("key", String);
            // No server networking credentials are returned to the browser.
            if key.starts_with("proxy_") { Value::Null } else { check_key(&key)?; json!(settings::get_setting(p,&key).await?) }
        },
        "set_setting" => {
            let key = read!("key", String); check_key(&key)?;
            let value = read!("value", String);
            let _write = state.writes.lock().await;
            settings::set_setting(p,&key,&value).await?;
            apply_setting(&key,Some(&value)); Value::Null
        },
        "get_watch_history" | "get_music_history" => {
            let limit = read!("limit", i64).clamp(1, 10_000);
            let offset = read!("offset", i64).max(0);
            if command == "get_music_history" { json!(watch_history::get_music_history(p,limit,offset).await?) }
            else { json!(watch_history::get_watch_history(p,limit,offset).await?) }
        },
        "add_watch_record" | "restore_watch_record" => {
            let record = read!("record", WatchHistoryRecord); check_record(&record)?;
            let _write = state.writes.lock().await;
            watch_history::upsert_watch_record(p,&record).await?; Value::Null
        },
        "add_watch_records_bulk" => {
            let records = read!("records", Vec<WatchHistoryRecord>);
            if records.len() > 2000 { return Err(AppError::Validation("Too many records in one import".into())); }
            for r in &records { check_record(r)?; }
            let _write = state.writes.lock().await;
            watch_history::upsert_watch_records_bulk(p,&records).await?; Value::Null
        },
        "delete_watch_record" => {
            let id = read!("videoId", String); validate_video_id(&id)?;
            let _write = state.writes.lock().await;
            watch_history::delete_watch_record(p,&id).await?; Value::Null
        },
        "get_watch_records" => {
            let ids = read!("videoIds", Vec<String>);
            if ids.len() > 10_000 { return Err(AppError::Validation("Too many video IDs".into())); }
            for id in &ids { validate_video_id(id)?; }
            json!(watch_history::get_watch_records(p,&ids).await?)
        },
        "get_watch_progress" => json!(watch_history::watch_progress(p).await?),
        "get_watch_history_years" => json!(watch_history::watch_history_years(p).await?),
        "delete_short_watch_records" => {
            let _write = state.writes.lock().await;
            watch_history::delete_short_watch_records(p).await?; Value::Null
        },
        "clear_watch_history" => {
            let _write = state.writes.lock().await;
            watch_history::clear_watch_history(p).await?; Value::Null
        },
        "watch_history_settled" => Value::Null, // Linked-device sync is not connected yet.
        "get_unplayable_video_ids" => json!(crate::db::unplayable_videos::all(p).await?),
        _ => return Ok(None),
    };
    Ok(Some(result))
}
