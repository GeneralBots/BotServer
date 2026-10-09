pub(crate) use anyhow::{anyhow, Context, Result};
pub(crate) use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
pub(crate) use reqwest::{Certificate, Client, ClientBuilder};
pub(crate) use ring::digest::{digest, SHA256};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::sync::{Arc, RwLock};
pub(crate) use std::time::Duration;
pub(crate) use tracing::{debug, error, info, warn};
pub(crate) use x509_parser::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertPinningConfig {
    pub enabled: bool,

    pub pins: HashMap<String, Vec<PinnedCert>>,

    pub require_pins: bool,

    pub allow_backup_pins: bool,

    pub report_only: bool,

    pub config_path: Option<PathBuf>,

    pub cache_ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinnedCert {
    pub hostname: String,

    pub fingerprint: String,

    pub description: Option<String>,

    pub is_backup: bool,

    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,

    pub pin_type: PinType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
pub enum PinType {
    #[default]
    Leaf,

    Intermediate,

    Root,
}


#[derive(Debug, Clone)]
pub struct PinValidationResult {
    pub valid: bool,

    pub hostname: String,

    pub matched_pin: Option<String>,

    pub actual_fingerprint: String,

    pub error: Option<String>,

    pub backup_match: bool,
}

#[derive(Debug)]
pub struct CertPinningManager {
    config: Arc<RwLock<CertPinningConfig>>,
    validation_cache: Arc<RwLock<HashMap<String, (PinValidationResult, std::time::Instant)>>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PinningStats {
    pub enabled: bool,
    pub total_hosts: usize,
    pub total_pins: usize,
    pub backup_pins: usize,
    pub expiring_soon: usize,
    pub report_only: bool,
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

pub use part6::*;
