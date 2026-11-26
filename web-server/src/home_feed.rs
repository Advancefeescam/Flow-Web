//! Original Home-feed data helpers used by FlowNeuro's taste and channel tools.
#[path = "../../src-tauri/src/services/home_feed/card.rs"]
pub mod card;
#[path = "../../src-tauri/src/services/home_feed/channel_memory.rs"]
pub mod channel_memory;
#[path = "../../src-tauri/src/services/home_feed/filters.rs"]
pub mod filters;
#[path = "../../src-tauri/src/services/home_feed/seeds.rs"]
pub mod seeds;
#[path = "../../src-tauri/src/services/home_feed/inputs.rs"]
pub mod inputs;
#[path = "../../src-tauri/src/services/home_feed/neighbours.rs"]
pub mod neighbours;

/// The taste summary only needs channel names that have appeared in saved subscription uploads.
pub mod cache {
    use crate::errors::AppResult;
    use sqlx::SqlitePool;

    pub async fn subscription_channel_names(pool: &SqlitePool) -> AppResult<Vec<(String, String)>> {
        Ok(sqlx::query_as::<_, (String, String)>(
            "SELECT DISTINCT json_extract(card, '$.channelId'), json_extract(card, '$.channelName') \
             FROM subscription_uploads \
             WHERE json_extract(card, '$.channelId') IS NOT NULL AND json_extract(card, '$.channelName') != ''",
        ).fetch_all(pool).await?)
    }
}
