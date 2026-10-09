pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{info, trace};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub url: String,
    pub model_path: String,
    pub api_key: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[derive(Default)]
pub enum RoutingStrategy {
    #[default]
    Manual,
    Auto,
    LoadBalanced,
    Fallback,
}


#[derive(Debug, Clone)]
pub struct ModelRouter {
    pub models: HashMap<String, ModelConfig>,
    pub default_model: String,
    pub routing_strategy: RoutingStrategy,
}

use std::sync::Arc;
use uuid::Uuid;

mod part1;
mod part2;
mod part3;

pub use part1::*;
pub use part2::*;
use part3::*;
