//! A small browser-owned Home feed coordinator. Search and ranking still use Flow's
//! original services; the browser polls snapshots instead of relying on Tauri events.
use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};

use serde_json::{json, Value};
use tokio::sync::Mutex;

use crate::{
    AppState,
        db::settings,
        errors::{AppError, AppResult},
    models::search::SearchVideosRequest,
    services::recommendation_service::DiscoverySurface,
};

#[derive(Clone)]
pub struct HomeService {
    snapshot: Arc<Mutex<Value>>,
    generation: Arc<Mutex<u64>>,
    pages: Arc<Mutex<usize>>,
}

impl HomeService {
    pub fn new() -> Self {
        Self {
            snapshot: Arc::new(Mutex::new(json!({
                "generation": 0, "cards": [], "origins": {}, "loading": false,
                "loadingMore": false, "hasMore": false, "loadMoreFailed": false, "error": null
            }))),
            generation: Arc::new(Mutex::new(0)),
            pages: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn snapshot(&self) -> Value { self.snapshot.lock().await.clone() }

    pub async fn open(&self, state: &AppState) -> AppResult<Value> {
        let current = self.snapshot().await;
        if current["cards"].as_array().is_some_and(|cards| !cards.is_empty()) || current["loading"] == true {
            return Ok(current);
        }
        self.load(state, true).await?;
        Ok(self.snapshot().await)
    }

    pub async fn refresh(&self, state: &AppState) -> AppResult<Value> {
        self.load(state, true).await?;
        Ok(self.snapshot().await)
    }

    pub async fn load_more(&self, state: &AppState) -> AppResult<Value> {
        let current = self.snapshot().await;
        if current["loading"] == true || current["loadingMore"] == true || current["hasMore"] != true {
            return Ok(current);
        }
        self.load(state, false).await?;
        Ok(self.snapshot().await)
    }

    pub async fn clear(&self) -> AppResult<()> {
        let mut generation = self.generation.lock().await;
        *generation += 1;
        *self.pages.lock().await = 0;
        *self.snapshot.lock().await = json!({
            "generation": *generation, "cards": [], "origins": {}, "loading": false,
            "loadingMore": false, "hasMore": false, "loadMoreFailed": false, "error": null
        });
        Ok(())
    }

    async fn load(&self, state: &AppState, reset: bool) -> AppResult<()> {
        if reset { *self.pages.lock().await = 0; }
        let generation = {
            let mut generation = self.generation.lock().await;
            *generation += 1;
            *generation
        };
        {
            let mut snapshot = self.snapshot.lock().await;
            snapshot["generation"] = json!(generation);
            snapshot["loading"] = json!(reset);
            snapshot["loadingMore"] = json!(!reset);
            snapshot["error"] = Value::Null;
        }

        let queries = match state.recommendations
            .generate_discovery_queries(DiscoverySurface::Home, reset).await {
            Ok(queries) => queries,
            Err(error) => {
                self.finish_failed(generation, "local").await;
                return Err(error);
            }
        };
        let queries: Vec<String> = queries.into_iter().take(5).collect();
        let results = futures_util::future::join_all(queries.iter().map(|query| async {
            let request = SearchVideosRequest { query: query.clone(), ..Default::default() };
            tokio::time::timeout(Duration::from_secs(20), state.youtube.search_videos(request)).await
        })).await;

        let mut candidates = Vec::new();
        let mut origin_by_id = HashMap::new();
        for (query, result) in queries.iter().zip(results) {
            if let Ok(Ok(page)) = result {
                let ids: Vec<String> = page.items.iter().map(|video| video.id.clone()).collect();
                state.recommendations.report_discovery_results(query, &ids).await;
                for video in page.items {
                    origin_by_id.entry(video.id.clone()).or_insert_with(|| json!({"kind":"search","detail":query}));
                    candidates.push(video);
                }
            }
        }
        candidates.sort_by(|a, b| a.id.cmp(&b.id));
        candidates.dedup_by(|a, b| a.id == b.id);
        let subscriptions = match settings::get_setting(&state.pool, "subscriptions").await {
            Ok(Some(raw)) => serde_json::from_str::<Vec<Value>>(&raw).unwrap_or_default().into_iter()
                .filter_map(|entry| entry["id"].as_str().map(str::to_owned)).collect(),
            _ => HashSet::new(),
        };
        let ranked = match state.recommendations.rank_candidates(candidates, subscriptions, 0.0).await {
            Ok(ranked) => ranked,
            Err(error) => {
                self.finish_failed(generation, "local").await;
                return Err(error);
            }
        };
        let cards: Vec<Value> = ranked.iter().map(|video| {
            let mut value = serde_json::to_value(video).unwrap_or(Value::Null);
            value["publishedAtMs"] = json!(video.published_at_ms.unwrap_or(0));
            value
        }).collect();
        let origins: serde_json::Map<String, Value> = ranked.iter().filter_map(|video| {
            origin_by_id.get(&video.id).cloned().map(|origin| (video.id.clone(), origin))
        }).collect();

        let mut snapshot = self.snapshot.lock().await;
        // A refresh may have superseded this slow search.
        if snapshot["generation"].as_u64() != Some(generation) { return Ok(()); }
        if reset {
            snapshot["cards"] = json!(cards);
            snapshot["origins"] = Value::Object(origins);
        } else {
            let mut existing = snapshot["cards"].as_array().cloned().unwrap_or_default();
            let seen: HashSet<String> = existing.iter().filter_map(|v| v["id"].as_str().map(str::to_owned)).collect();
            let start = existing.len();
            existing.extend(cards.into_iter().filter(|v| v["id"].as_str().is_some_and(|id| !seen.contains(id))));
            snapshot["cards"] = json!(existing);
            if let Some(target) = snapshot["origins"].as_object_mut() { target.extend(origins); }
            snapshot["appendStart"] = json!(start);
        }
        let page = {
            let mut pages = self.pages.lock().await;
            *pages = if reset { 1 } else { *pages + 1 };
            *pages
        };
        snapshot["loading"] = json!(false);
        snapshot["loadingMore"] = json!(false);
        snapshot["hasMore"] = json!(ranked_len_has_more(&snapshot) && page < 6);
        snapshot["loadMoreFailed"] = json!(false);
        snapshot["error"] = if snapshot["cards"].as_array().is_some_and(Vec::is_empty) { json!("network") } else { Value::Null };
        Ok(())
    }

    async fn finish_failed(&self, generation: u64, kind: &str) {
        let mut snapshot = self.snapshot.lock().await;
        if snapshot["generation"].as_u64() != Some(generation) { return; }
        snapshot["loading"] = json!(false);
        snapshot["loadingMore"] = json!(false);
        snapshot["loadMoreFailed"] = json!(true);
        snapshot["error"] = json!(kind);
    }
}

fn ranked_len_has_more(snapshot: &Value) -> bool {
    snapshot["cards"].as_array().is_some_and(|cards| cards.len() > 0)
}

pub async fn dispatch(state: &AppState, command: &str, args: &Value) -> AppResult<Option<Value>> {
    let value = match command {
        "home_feed_open" => Some(state.home.open(state).await?),
        "home_feed_refresh" => Some(state.home.refresh(state).await?),
        "home_feed_snapshot" => Some(state.home.snapshot().await),
        "home_feed_viewport" => {
            let last_visible = args["lastVisible"].as_u64().unwrap_or(0) as usize;
            let snapshot = state.home.snapshot().await;
            let length = snapshot["cards"].as_array().map_or(0, Vec::len);
            if length > 0 && last_visible.saturating_add(5) >= length { Some(state.home.load_more(state).await?) }
            else { Some(Value::Null) }
        },
        "home_feed_retry_more" => Some(state.home.load_more(state).await?),
        "home_feed_visible" => Some(Value::Null),
        "home_feed_clear" => { state.home.clear().await?; Some(Value::Null) },
        "home_feed_chips" => Some(json!({"interests":[],"hasMixes":false,"hasWatched":false,"empty":[]})),
        "home_chip_feed" => Some(json!({"cards":[],"mixes":[],"failed":false})),
        _ => return Ok(None),
    };
    Ok(value)
}
