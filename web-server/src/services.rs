#[path = "../../src-tauri/src/services/recommendation_service.rs"]
pub mod recommendation_service;

#[path = "../../src-tauri/src/services/shorts_service.rs"]
pub mod shorts_service;

#[path = "../../src-tauri/src/services/shorts_engine.rs"]
pub mod shorts_engine;

#[path = "../../src-tauri/src/services/home_engine.rs"]
pub mod home_engine;

#[path = "../../src-tauri/src/services/taste_service.rs"]
pub mod taste_service;

#[path = "../../src-tauri/src/services/blocked_channel_names.rs"]
pub mod blocked_channel_names;

pub use crate::subscription_feed;

pub mod youtube_service {
    pub use crate::youtube_service::*;
}

pub use crate::home_feed;
