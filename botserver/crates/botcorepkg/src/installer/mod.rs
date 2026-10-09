//! Package manager: component registry, allowed shell helpers and installer URLs.

pub(crate) use crate::component::ComponentConfig;
pub(crate) use crate::os::detect_os;
pub(crate) use crate::{InstallMode, OsType};
pub(crate) use crate::installer_regs;
pub(crate) use crate::installer_regs2;
pub(crate) use crate::installer_vault;
pub(crate) use crate::installer_vault2;
pub(crate) use botlib::security::SafeCommand;
pub(crate) use anyhow::Result;
pub(crate) use log::{info, trace, warn};
pub(crate) use serde::Deserialize;
pub(crate) use std::collections::HashMap;
pub(crate) use std::path::PathBuf;

#[derive(Deserialize, Debug)]
struct ComponentEntry {
    url: String,
    #[serde(default)]
    url_win: Option<String>,
}

#[derive(Deserialize, Debug)]
struct ThirdPartyConfig {
    components: HashMap<String, ComponentEntry>,
    #[serde(default)]
    models: HashMap<String, ComponentEntry>,
}

static THIRDPARTY_CONFIG: std::sync::OnceLock<ThirdPartyConfig> = std::sync::OnceLock::new();

#[derive(Debug)]
pub struct PackageManager {
    pub mode: InstallMode,
    pub os_type: OsType,
    pub base_path: PathBuf,
    pub tenant: String,
    pub components: HashMap<String, ComponentConfig>,
}

mod component_entry;
mod urls;
mod shell;
mod llama;
mod manager_new;
mod manager_run;

pub use urls::*;
pub use shell::*;
pub use llama::*;
