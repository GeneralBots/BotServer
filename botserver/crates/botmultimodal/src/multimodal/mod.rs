pub(crate) use botcore::InternalUrls;
pub(crate) use botlib::security::create_tls_client;
pub(crate) use log::{info, trace};
pub(crate) use reqwest::Client;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use uuid::Uuid;

pub trait ConfigProvider {
    fn get_config(&self, bot_id: &Uuid, key: &str, default: Option<&str>) -> Option<String>;
}

#[derive(Debug, Clone)]
pub struct BotModelsConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub api_key: String,
    pub use_https: bool,
}

#[derive(Debug, Clone)]
pub struct ImageGeneratorConfig {
    pub model: String,
    pub steps: u32,
    pub width: u32,
    pub height: u32,
    pub gpu_layers: u32,
    pub batch_size: u32,
}

#[derive(Debug, Clone)]
pub struct VideoGeneratorConfig {
    pub model: String,
    pub frames: u32,
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub gpu_layers: u32,
    pub batch_size: u32,
}

#[derive(Debug, Serialize)]
pub struct ImageGenerateRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guidance_scale: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct VideoGenerateRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_frames: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SpeechGenerateRequest {
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GenerationResponse {
    pub status: String,
    pub file_path: Option<String>,
    pub generation_time: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DescribeResponse {
    pub description: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct VideoDescribeResponse {
    pub description: String,
    pub frame_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
pub struct SpeechToTextResponse {
    pub text: String,
    pub language: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug)]
pub struct BotModelsClient {
    client: Client,
    config: BotModelsConfig,
    image_config: ImageGeneratorConfig,
    video_config: VideoGeneratorConfig,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
