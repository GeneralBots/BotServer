mod api;
mod auth;
mod b64;
mod blobstore;
mod catalog;
mod install;
pub mod models;
pub mod schema;
mod publish;
pub mod seed;
mod seed_data;

use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

pub use botlib::db_pool::DbPool;

/// Async consent hook receiving (bot_id, ConsentRequest json) and answering allow/deny.
pub type ConsentFuture = Pin<Box<dyn Future<Output = Result<bool, String>> + Send>>;
pub type ConsentChecker = Arc<dyn Fn(Uuid, Value) -> ConsentFuture + Send + Sync>;

#[derive(Clone)]
pub struct MarketplaceService {
    pub pool: DbPool,
    pub mc_bin: String,
    pub mc_alias: String,
    pub require_consent: bool,
    pub consent_checker: Option<ConsentChecker>,
}

impl MarketplaceService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            mc_bin: "mc".to_string(),
            mc_alias: "local".to_string(),
            require_consent: false,
            consent_checker: None,
        }
    }

    pub fn with_mc(mut self, mc_bin: &str, mc_alias: &str) -> Self {
        self.mc_bin = mc_bin.to_string();
        self.mc_alias = mc_alias.to_string();
        self
    }

    pub fn with_require_consent(mut self, require: bool) -> Self {
        self.require_consent = require;
        self
    }

    pub fn with_consent_checker(mut self, checker: Option<ConsentChecker>) -> Self {
        self.consent_checker = checker;
        self
    }
}

pub use api::configure_routes;
pub use models::{InstallBody, PackageRow, PublishBody, VersionRow};
pub use seed::{seed_if_empty, seed_starter_skills};
