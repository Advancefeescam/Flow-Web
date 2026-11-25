//! Keep the original schema and database helpers; no separate incompatible library format.
#[path = "../../src-tauri/src/db/device.rs"] pub mod device;
#[path = "../../src-tauri/src/db/settings.rs"] pub mod settings;
#[path = "../../src-tauri/src/db/watch_history.rs"] pub mod watch_history;
#[path = "../../src-tauri/src/db/library.rs"] pub mod library;
#[path = "../../src-tauri/src/db/music_content.rs"] pub mod music_content;
#[path = "../../src-tauri/src/db/music_home.rs"] pub mod music_home;
#[path = "../../src-tauri/src/db/known_shorts.rs"] pub mod known_shorts;
#[path = "../../src-tauri/src/db/unplayable_videos.rs"] pub mod unplayable_videos;
#[path = "../../src-tauri/src/db/neuro_stores.rs"] pub mod neuro_stores;
#[path = "../../src-tauri/src/db/seen_videos.rs"] pub mod seen_videos;
#[path = "../../src-tauri/src/db/recommendations.rs"] pub mod recommendations;

use std::{path::Path, time::Duration};
use sqlx::{SqlitePool, sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions}};
use crate::errors::{AppError, AppResult};

pub async fn open(dir: &Path) -> AppResult<SqlitePool> {
    let pool = SqlitePoolOptions::new().max_connections(4).connect_with(
        SqliteConnectOptions::new().filename(dir.join("flow-web.sqlite"))
        .create_if_missing(true).journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5)).foreign_keys(true)
    ).await?;
    sqlx::migrate!("../src-tauri/migrations").run(&pool).await
        .map_err(|e| AppError::Internal(format!("Database migration failed: {e}")))?;
    Ok(pool)
}
