//! Process/thread memory monitoring, checkpoints, component tracking and leak detection.

pub(crate) use log::{debug, info, trace};
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::{LazyLock, Mutex, RwLock};
pub(crate) use std::time::{Duration, Instant};
#[cfg(feature = "monitoring")]
pub(crate) use sysinfo::{Pid, ProcessesToUpdate, System};

static THREAD_REGISTRY: LazyLock<RwLock<HashMap<String, ThreadInfo>>> =
LazyLock::new(|| RwLock::new(HashMap::new()));

static COMPONENT_TRACKER: LazyLock<ComponentMemoryTracker> =
LazyLock::new(|| ComponentMemoryTracker::new(60));

#[derive(Debug, Clone)]
pub struct ThreadInfo {
pub name: String,
pub started_at: Instant,
pub last_activity: Instant,
pub activity_count: u64,
pub component: String,
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
pub rss_bytes: u64,
pub virtual_bytes: u64,
pub timestamp: Instant,
}

mod threads;
mod memory_stats;
mod jemalloc;
mod checkpoint;
mod component;
mod leak;
mod monitor;
mod tests;

pub use threads::*;
pub use jemalloc::*;
pub use checkpoint::*;
pub use component::*;
pub use leak::*;
pub use monitor::*;
