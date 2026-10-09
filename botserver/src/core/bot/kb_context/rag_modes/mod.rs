pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use log::{debug, info};
pub(crate) use uuid::Uuid;
pub(crate) use botcore::config::ConfigManager;
pub(crate) use botcore::kb::web_crawler::WebCrawler;
pub(crate) use botcore::shared::utils::DbPool;
pub(crate) use super::kb_context_search;
pub(crate) use super::KbSearchResult;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RagMode {
    Standard,
    Hybrid,
    Corrective,
    Graph,
    Agentic,
    Multimodal,
}

const RRF_K: f64 = 60.0;

mod part1;
mod part2;
mod part3;

pub use part1::*;
use part2::*;
use part3::*;
