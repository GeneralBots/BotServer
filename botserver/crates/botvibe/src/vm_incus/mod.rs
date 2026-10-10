//! Split from `vm_incus.rs` per #1443 (AGENTS.md 450-line rule).
//! #744 — Incus CLI driver for the VM lifecycle (host-side container ops).
//! Runs the `incus` CLI via the harness command guard so every invocation
//! is allowlisted and argument-validated; containers are visible through
//! `incus list` (Verifiable per #744 DoD).
//!
//! Dual-platform: on Linux the `incus` binary runs directly; on Windows the
//! same CLI runs inside an explicit WSL2 distro, with automatic first-run
//! provisioning of WSL2 + Debian + the Incus package. Set
//! `GBO_WSL_DISTRO` to select another installed distro.

mod health;
mod linux;
mod web_entry;
mod windows;
mod windows_2;
mod windows_3;
mod windows_4;

#[cfg(target_os = "windows")]
use std::sync::OnceLock;
use crate::harness::cmd::{run, GuardError, RunOutput};
use sha2::{Digest, Sha256};
#[cfg(target_os = "windows")]
use crate::harness::cmd::spawn_persistent;
use crate::vm_lifecycle::VmLifecycle;

pub(crate) use health::{HEALTH_PROBE_JS, checked_run};
pub(crate) use web_entry::{HEALTH_PROBE_PYTHON, resolve_web_entry};
#[cfg(test)]
pub(crate) use web_entry::{entry_resolution_tests};
#[cfg(target_os = "windows")]
pub(crate) use windows::{ensure_wsl_keepalive, windows_path_to_wsl, wsl_distro, wsl_exec_args};
