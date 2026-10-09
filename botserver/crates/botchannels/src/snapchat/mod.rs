//! Snapchat Marketing API Integration
//!
//! Provides Snap Ads management, audience targeting, and content publishing capabilities.
//! Supports OAuth 2.0 authentication flow for Snapchat Marketing API.

pub(crate) use crate::{ ChannelAccount, ChannelCredentials, ChannelError, ChannelProvider, ChannelType, PostContent, PostResult, };
pub(crate) use serde::{Deserialize, Serialize};

    /// Provider endpoints are compile-time constants: tokens are scoped to
    /// these hosts and must never be sent to a runtime-configured one.
    const API_BASE: &str = "https://adsapi.snapchat.com/v1";
    const OAUTH_BASE: &str = "https://accounts.snapchat.com";

    /// Create a creative

    /// Create a custom audience

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;

pub use part1::*;
pub use part5::*;
pub use part6::*;
pub use part7::*;
