pub(crate) use crate::package_manager::cache::{CacheResult, DownloadCache};
pub(crate) use crate::package_manager::component::{ComponentConfig, InstallResult};
use crate::shared::utils::{self, get_database_url_sync, parse_database_url};
pub(crate) use anyhow::{Context, Result};
pub(crate) use log::{error, info, trace, warn};
pub(crate) use std::io::Write;

use crate::package_manager::installer::PackageManager;
use crate::package_manager::InstallMode;
use crate::package_manager::OsType;
use botlib::security::command_guard::SafeCommand;
use reqwest::Client;
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
mod part7;
mod part8;
mod part9;

use part1::*;
use part9::*;
