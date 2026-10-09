pub(crate) use crate::package_manager::component::ComponentConfig;
pub(crate) use crate::package_manager::os::detect_os;
pub(crate) use crate::package_manager::{InstallMode, OsType};
pub(crate) use anyhow::{Context, Result};
pub(crate) use log::{info, trace, warn};
pub(crate) use std::io::Write;

#[derive(Debug)]
pub struct PackageManager {
    pub mode: InstallMode,
    pub os_type: OsType,
    pub base_path: PathBuf,
    pub tenant: String,
    pub components: HashMap<String, ComponentConfig>,
}

    /// Initialize Vault locally (non-LXC mode) and create .env file
    ///
    /// This function:
    /// 1. Checks if Vault is already initialized (via health endpoint or data dir)
    /// 2. If initialized but sealed, unseals with existing keys from vault-unseal-keys
    /// 3. If not initialized, runs `vault operator init` to get root token and unseal keys
    /// 4. Creates .env file with VAULT_ADDR and VAULT_TOKEN
    /// 5. Creates vault-unseal-keys file with proper permissions
    /// 6. Unseals Vault with 3 keys

    /// Seed default credentials into Vault KV2 after initialization

    /// Unseal Vault with 3 keys

use botlib::security::command_guard::SafeCommand;
use std::collections::HashMap;
use std::path::PathBuf;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part1::*;
