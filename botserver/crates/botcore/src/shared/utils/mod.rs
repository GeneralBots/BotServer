pub(crate) use anyhow::{Context, Result};
pub(crate) use diesel::Connection;
pub(crate) use diesel::{ r2d2::{ConnectionManager, Pool}, PgConnection, };
#[cfg(feature = "drive")]
pub(crate) use crate::config::DriveConfig;
#[cfg(feature = "progress-bars")]
pub(crate) use indicatif::{ProgressBar, ProgressStyle};
pub(crate) use log::{debug, warn};
pub(crate) use reqwest::{Certificate, Client};
pub(crate) use rhai::{Array, Dynamic};
pub(crate) use serde_json::Value;
pub(crate) use smartstring::SmartString;
pub(crate) use std::error::Error;
pub(crate) use std::time::Duration;
pub(crate) use tokio::fs::File as TokioFile;
pub(crate) use tokio::io::AsyncWriteExt;
pub(crate) use botsecurity_core::VaultConfigProvider;
pub(crate) use botcoresecrets::SecretsManager;

struct SecretsManagerVaultProvider(SecretsManager);

pub use botlib::db_pool::DbPool;

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
