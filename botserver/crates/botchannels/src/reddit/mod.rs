pub(crate) use chrono::{DateTime, Utc};
pub(crate) use reqwest::Client;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub user_agent: String,
    pub username: Option<String>,
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditPost {
    pub id: String,
    pub subreddit: String,
    pub title: String,
    pub selftext: Option<String>,
    pub url: Option<String>,
    pub author: String,
    pub score: i64,
    pub num_comments: u64,
    pub created_utc: f64,
    pub permalink: String,
    pub is_self: bool,
    pub link_flair_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditComment {
    pub id: String,
    pub body: String,
    pub author: String,
    pub score: i64,
    pub created_utc: f64,
    pub parent_id: String,
    pub link_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subreddit {
    pub name: String,
    pub display_name: String,
    pub title: String,
    pub description: Option<String>,
    pub subscribers: u64,
    pub public_description: Option<String>,
    pub subreddit_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitPostRequest {
    pub subreddit: String,
    pub title: String,
    pub kind: PostKind,
    pub content: String,
    pub flair_id: Option<String>,
    pub flair_text: Option<String>,
    pub nsfw: bool,
    pub spoiler: bool,
    pub send_replies: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PostKind {
    #[serde(rename = "self")]
    Text,
    Link,
    Image,
    Video,
    Poll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitCommentRequest {
    pub parent_id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedditUser {
    pub id: String,
    pub name: String,
    pub created_utc: f64,
    pub link_karma: i64,
    pub comment_karma: i64,
    pub is_gold: bool,
    pub is_mod: bool,
    pub has_verified_email: bool,
    pub icon_img: Option<String>,
}

pub struct RedditChannel {
    config: RedditConfig,
    http_client: Client,
    tokens: Arc<RwLock<Option<RedditTokens>>>,
    base_url: String,
    oauth_url: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PostSort {
    Hot,
    New,
    Top,
    Rising,
    Controversial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VoteDirection {
    Up,
    Down,
    None,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: u64,
    scope: String,
}

#[derive(Debug, Deserialize)]
struct SubmitResponse {
    json: SubmitJsonResponse,
}

#[derive(Debug, Deserialize)]
struct SubmitJsonResponse {
    errors: Option<Vec<Vec<String>>>,
    data: Option<SubmitDataResponse>,
}

#[derive(Debug, Deserialize)]
struct SubmitDataResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct CommentResponse {
    json: CommentJsonResponse,
}

#[derive(Debug, Deserialize)]
struct CommentJsonResponse {
    errors: Option<Vec<Vec<String>>>,
    data: Option<CommentDataResponse>,
}

#[derive(Debug, Deserialize)]
struct CommentDataResponse {
    things: Vec<ThingWrapper>,
}

#[derive(Debug, Deserialize)]
struct ThingWrapper {
    data: Option<ThingData>,
}

#[derive(Debug, Deserialize)]
struct ThingData {
    id: Option<String>,
    body: Option<String>,
    author: Option<String>,
    score: Option<i64>,
    created_utc: Option<f64>,
    parent_id: Option<String>,
    link_id: Option<String>,
    subreddit: Option<String>,
    title: Option<String>,
    selftext: Option<String>,
    url: Option<String>,
    num_comments: Option<u64>,
    permalink: Option<String>,
    is_self: Option<bool>,
    link_flair_text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ListingResponse {
    data: ListingData,
}

#[derive(Debug, Deserialize)]
struct ListingData {
    children: Vec<ThingWrapper>,
}

#[derive(Debug, Deserialize)]
struct AboutResponse {
    data: AboutData,
}

#[derive(Debug, Deserialize)]
struct AboutData {
    name: Option<String>,
    display_name: Option<String>,
    title: Option<String>,
    description: Option<String>,
    subscribers: Option<u64>,
    public_description: Option<String>,
    subreddit_type: Option<String>,
}

#[derive(Debug, Clone)]
pub enum RedditError {
    NetworkError(String),
    ApiError(String),
    ParseError(String),
    ConfigError(String),
    NotAuthenticated,
    PostNotFound,
    RateLimited,
}

mod part1;
mod part2;
mod part3;
mod part4;

pub use part4::*;
