//! Cached `Regex` compilation (#1493).
//!
//! This crate builds its patterns at runtime (a spreadsheet criterion such as
//! `"abc*"` becomes `^abc.*$`), so it cannot hold them in `static` items. The
//! memo below compiles each distinct pattern once and returns a cheap clone:
//! `regex::Regex` holds `Arc`s internally, so cloning never recompiles. The
//! map is cleared at the limit, so a workload with many distinct patterns
//! degrades to the previous behaviour instead of growing without bound.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const DYNAMIC_LIMIT: usize = 256;

/// Returns `None` for a pattern that does not compile, exactly like the
/// `if let Ok(re)` call sites this replaces.
pub(crate) fn dynamic(pattern: &str) -> Option<Regex> {
    static CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(guard) = cache.lock() {
        if let Some(cached) = guard.get(pattern) {
            return Some(cached.clone());
        }
    }
    let compiled = Regex::new(pattern).ok()?;
    if let Ok(mut guard) = cache.lock() {
        if guard.len() >= DYNAMIC_LIMIT {
            guard.clear();
        }
        guard.insert(pattern.to_string(), compiled.clone());
    }
    Some(compiled)
}
