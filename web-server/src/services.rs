#[path = "../../src-tauri/src/services/recommendation_service.rs"]
pub mod recommendation_service;

#[path = "../../src-tauri/src/services/shorts_service.rs"]
pub mod shorts_service;

#[path = "../../src-tauri/src/services/shorts_engine.rs"]
pub mod shorts_engine;

pub mod youtube_service {
    pub use crate::youtube_service::*;
}

pub mod home_feed {
    pub mod channel_memory {
        use crate::errors::AppResult;
        use sqlx::SqlitePool;

        pub async fn record_rejection(
            pool: &SqlitePool,
            channel_id: &str,
            name: &str,
            now_ms: u64,
        ) -> AppResult<()> {
            if channel_id.is_empty() {
                return Ok(());
            }
            sqlx::query(
                "INSERT INTO home_channel_memory (channel_id, name, rejected_at, uploads) VALUES (?, ?, ?, '[]')
                 ON CONFLICT(channel_id) DO UPDATE SET rejected_at = excluded.rejected_at, uploads = '[]'",
            )
            .bind(channel_id)
            .bind(name)
            .bind(i64::try_from(now_ms).unwrap_or(i64::MAX))
            .execute(pool)
            .await?;
            Ok(())
        }
    }
}
