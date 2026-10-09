// Bootstrap manager implementation

//! Bootstrap manager: stack lifecycle, install, template sync and directory helpers.

pub(crate) use crate::bootstrap::bootstrap_types::{BootstrapManager, BootstrapProgress};
pub(crate) use crate::bootstrap::bootstrap_utils::{ alm_health_check, cache_health_check, drive_health_check, safe_pkill, tables_health_check, vault_health_check, vector_db_health_check, zitadel_health_check, };
pub(crate) use crate::package_manager::{InstallMode, PackageManager};
pub(crate) use anyhow::Context;
pub(crate) use diesel::RunQueryDsl;
pub(crate) use log::{debug, info, warn};
pub(crate) use std::path::Path;
pub(crate) use std::time::Duration;
pub(crate) use tokio::time::sleep;
pub(crate) use uuid::Uuid;
pub use super::instance::{check_single_instance, release_instance_lock};
pub use super::vault::{get_db_password_from_vault, has_installed_stack, reset_vault_only};
pub(crate) use crate::config::AppConfig;
pub(crate) use crate::shared::utils::get_stack_path;
pub(crate) use botlib::security::command_guard::SafeCommand;
pub(crate) use std::path::PathBuf;

    /// Check system status

    /// Find the directory holding template bots: the cloned templates repo
    /// (`work/templates`) takes precedence, falling back to the bundled
    /// `bottemplates/bots` directory shipped with the workspace.

// Standalone functions for backward compatibility

mod lifecycle;
mod install;
mod templates;
mod helpers;

use helpers::*;
