pub(crate) use axum::{ extract::State, http::StatusCode, Json, };
pub(crate) use diesel::prelude::*;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;
pub(crate) use crate::schema::crm_contacts;
pub(crate) use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentGenerationRequest {
    pub channel: String,
    pub goal: String,
    pub audience_description: String,
    pub template_variables: Option<serde_json::Value>,
    pub tone: Option<String>,
    pub length: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentGenerationResult {
    pub subject: Option<String>,
    pub body: String,
    pub headline: Option<String>,
    pub cta: Option<String>,
    pub suggested_images: Vec<String>,
    pub variations: Vec<ContentVariation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentVariation {
    pub name: String,
    pub body: String,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationRequest {
    pub template: String,
    pub contact_id: Uuid,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalizationResult {
    pub personalized_content: String,
    pub variables_used: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestRequest {
    pub campaign_id: Uuid,
    pub variations: Vec<ABTestVariation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestVariation {
    pub name: String,
    pub subject: Option<String>,
    pub body: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTestResult {
    pub variation_id: String,
    pub opens: i64,
    pub clicks: i64,
    pub open_rate: f64,
    pub click_rate: f64,
    pub winner: bool,
}

const DEFAULT_TONE: &str = "professional";
const DEFAULT_LENGTH: &str = "medium";

mod body;

pub use body::*;
