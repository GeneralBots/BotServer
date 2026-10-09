// ProtectionInstaller/VerifyResult are in botsecurity which depends on botcore
// To break the cycle, security protection functions stay in botserver layer

pub(crate) use crate::package_manager::certs_utils;
pub(crate) use botcoresecrets::{SecretPaths, SecretsManager};
pub(crate) use crate::package_manager::{get_all_components, InstallMode, PackageManager};

const VERSION: &str = env!("CARGO_PKG_VERSION");

use botlib::security::command_guard::SafeCommand;
use anyhow::Result;
use rand::Rng;
use std::collections::HashMap;
use std::env;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;

use part1::*;
pub use part2::*;
use part3::*;
use part4::*;
use part5::*;
use part6::*;
