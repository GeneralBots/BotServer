//! YouTube Data API v3 Provider Implementation
//!
//! Provides video upload, community posts, and channel management capabilities.
//! Supports OAuth 2.0 authentication flow.

pub(crate) use crate::{ ChannelAccount, ChannelCredentials, ChannelError, ChannelProvider, ChannelType, PostContent, PostResult, };
pub(crate) use super::types::*;
pub(crate) use super::models::VideoMetadata;
pub(crate) use super::client::parse_error_response;

/// YouTube API provider for video uploads and community posts
pub struct YouTubeProvider {
    client: reqwest::Client,
    api_base_url: String,
    upload_base_url: String,
    oauth_base_url: String,
}

mod part1;
mod part2;
mod part3;
mod part4;
