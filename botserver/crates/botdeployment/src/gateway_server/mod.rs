//! Deploy Gateway Server — receives requests from alm-ci and performs
//! privileged operations on the host (Incus container management, Caddy config).
//!
//! This server validates the X-Deploy-Key header and never exposes
//! Forgejo tokens, SSH keys, or Incus socket access to callers.

pub(crate) use axum::{ extract::{Path, State}, http::{HeaderMap, StatusCode}, routing::{get, post}, Json, Router, };
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::RwLock;
pub(crate) use super::types::*;

#[derive(Debug, Clone, Deserialize)]
pub struct GatewayState {
    #[serde(default)]
    deploy_keys: Vec<String>,
    pub(crate) incus_socket: String,
    caddy_api_url: String,
    sites_root: String,
}

#[derive(Debug)]
pub struct ContainerRegistry {
    containers: RwLock<HashMap<String, ContainerInfo>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerInfo {
    pub name: String,
    pub org: String,
    pub project_type: ProjectType,
    pub status: String,
    pub url: String,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub(crate) use part1::*;
pub use part4::*;
use part5::*;
