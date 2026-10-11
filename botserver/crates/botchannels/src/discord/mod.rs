pub(crate) use crate::{ ChannelAccount, ChannelCredentials, ChannelError, ChannelProvider, ChannelType, PostContent, PostResult, };
pub(crate) use serde::{Deserialize, Serialize};

pub struct DiscordProvider {
    client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Serialize)]
pub struct MessageContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<DiscordEmbed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tts: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct WebhookContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avatar_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<DiscordEmbed>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tts: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DiscordEmbed {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<EmbedImage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumbnail: Option<EmbedImage>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    fields: Vec<EmbedField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    footer: Option<EmbedFooter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbedImage {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbedField {
    name: String,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    inline: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EmbedFooter {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    icon_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordMessage {
    pub id: String,
    pub channel_id: String,
    #[serde(default)]
    pub content: String,
    pub timestamp: String,
    pub author: Option<DiscordUser>,
}

#[derive(Debug, Deserialize)]
pub struct DiscordUser {
    pub id: String,
    pub username: String,
    pub discriminator: String,
    #[serde(default)]
    pub bot: bool,
}

#[derive(Debug, Serialize)]
struct CreateDMChannel {
    recipient_id: String,
}

pub struct DiscordBotConfig {
    pub token: String,
    pub application_id: String,
    pub guild_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct GuildChannel {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub channel_type: u8,
    pub guild_id: Option<String>,
    pub position: Option<i32>,
    pub topic: Option<String>,
}

mod part1;
mod part2;
mod part3;
mod part4;
