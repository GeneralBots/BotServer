//! Artifact download facade: fetch, extract and install components.

pub(crate) use crate::cache::{CacheResult, DownloadCache};
pub(crate) use crate::component::ComponentConfig;
pub(crate) use crate::db_utils::{get_database_url_sync, parse_database_url};
pub(crate) use crate::OsType;
pub(crate) use anyhow::{Context, Result};
pub(crate) use botlib::security::SafeCommand;
pub(crate) use log::{error, info, trace, warn};
pub(crate) use reqwest::Client;
pub(crate) use std::path::PathBuf;

mod preflight;
mod download;
mod install_env;
mod install_run;
mod archive;
mod finalize;

use preflight::*;
pub use download::*;
pub use install_env::*;
pub use install_run::*;
pub use archive::*;
pub use finalize::*;
