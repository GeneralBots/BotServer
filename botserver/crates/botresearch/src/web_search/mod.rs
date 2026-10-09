pub(crate) use axum::{ extract::{Query, State}, response::{Html, IntoResponse}, routing::{get, post}, Json, Router, };
pub(crate) use chrono::{DateTime, Utc};
pub(crate) use log::{debug, info};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::fmt::Write;
pub(crate) use std::sync::Arc;
pub(crate) use std::time::Duration;
pub(crate) use crate::ResearchState;

pub const ROUTE_RESEARCH_WEB_SEARCH: &str = "/api/ui/research/web/search";
pub const ROUTE_RESEARCH_WEB_SUMMARIZE: &str = "/api/ui/research/web/summarize";
pub const ROUTE_RESEARCH_WEB_DEEP: &str = "/api/ui/research/web/deep";
pub const ROUTE_RESEARCH_WEB_HISTORY: &str = "/api/ui/research/web/history";
pub const ROUTE_RESEARCH_WEB_INSTANT: &str = "/api/ui/research/web/instant";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchRequest {
    pub query: String,
    pub max_results: Option<usize>,
    pub region: Option<String>,
    pub safe_search: Option<bool>,
    pub time_range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
    pub favicon: Option<String>,
    pub published_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchResponse {
    pub results: Vec<WebSearchResult>,
    pub query: String,
    pub total_results: usize,
    pub search_time_ms: u64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeRequest {
    pub query: String,
    pub results: Vec<WebSearchResult>,
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummarizeResponse {
    pub summary: String,
    pub citations: Vec<Citation>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub index: usize,
    pub title: String,
    pub url: String,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchRequest {
    pub query: String,
    pub depth: Option<usize>,
    pub max_sources: Option<usize>,
    pub follow_links: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepResearchResponse {
    pub answer: String,
    pub sources: Vec<WebSearchResult>,
    pub citations: Vec<Citation>,
    pub related_queries: Vec<String>,
    pub confidence: f32,
    pub research_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryEntry {
    pub id: String,
    pub query: String,
    pub results_count: usize,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHistoryQuery {
    pub page: Option<i32>,
    pub per_page: Option<i32>,
}

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
use part3::*;
