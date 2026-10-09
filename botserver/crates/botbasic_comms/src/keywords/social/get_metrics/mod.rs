//! Social media metrics keywords and per-platform metric fetchers.

pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use log::{debug, trace};
pub(crate) use rhai::{Dynamic, Engine, Map};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::Value;
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PostEngagement {
    pub likes: i64,
    pub comments: i64,
    pub shares: i64,
    pub views: i64,
    pub clicks: i64,
    pub reach: i64,
}

impl PostEngagement {
    pub fn to_dynamic(&self) -> Dynamic {
        let mut map = Map::new();
        map.insert("likes".into(), Dynamic::from(self.likes));
        map.insert("comments".into(), Dynamic::from(self.comments));
        map.insert("shares".into(), Dynamic::from(self.shares));
        map.insert("views".into(), Dynamic::from(self.views));
        map.insert("clicks".into(), Dynamic::from(self.clicks));
        map.insert("reach".into(), Dynamic::from(self.reach));
        Dynamic::from(map)
    }
}

mod registration;
mod credentials;
mod fetch;
mod tests;

pub use registration::*;
use credentials::*;
use fetch::*;
