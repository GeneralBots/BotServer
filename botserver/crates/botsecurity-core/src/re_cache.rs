//! Cached `Regex` compilation (#1493).
//!
//! The patterns in this crate are constants, so each one is parsed and
//! compiled once per process instead of once per call. Following
//! `botconnectors::redact`, a pattern that fails to compile (impossible for
//! the literals used here) is logged and reported as `None`; the caller skips
//! that step instead of panicking inside the running server (#1368).

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Compile a constant pattern once per call site.
///
/// Expands to a block-scoped `OnceLock`, so the pattern stays beside the code
/// that uses it while every call site keeps its own cached `Regex`.
macro_rules! cached_re {
    ($pattern:expr) => {{
        static RE: ::std::sync::OnceLock<::core::option::Option<::regex::Regex>> =
            ::std::sync::OnceLock::new();
        RE.get_or_init(|| $crate::re_cache::compile($pattern)).as_ref()
    }};
}

pub(crate) use cached_re;

/// `cached_re!` for call sites that need an owned `Regex` (e.g. a struct
/// field). Cloning a `Regex` only bumps the `Arc`s it already holds.
macro_rules! cached_re_owned {
    ($pattern:expr) => {
        $crate::re_cache::cached_re!($pattern).cloned()
    };
}

pub(crate) use cached_re_owned;

pub(crate) fn compile(pattern: &'static str) -> Option<Regex> {
    match Regex::new(pattern) {
        Ok(re) => Some(re),
        Err(e) => {
            log::error!(
                "{}: pattern failed to compile ({pattern}): {e}",
                module_path!()
            );
            None
        }
    }
}

/// Patterns assembled at runtime (user text, cell criteria, workbook part
/// names) cannot be `static`. This memo compiles each distinct pattern once
/// and returns a cheap clone: `regex::Regex` holds `Arc`s internally, so
/// cloning never recompiles. The map is cleared at the limit, so a workload
/// with many distinct patterns degrades to the previous behaviour instead of
/// growing without bound.
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
