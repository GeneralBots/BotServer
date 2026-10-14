//! Cached `Regex` compilation (#1493).
//!
//! The HTML/regex step patterns in this crate are constants, so each one is
//! parsed and compiled once per process instead of once per document.
//! Following `botconnectors::redact`, a pattern that fails to compile
//! (impossible for the literals used here) is logged and reported as `None`;
//! the caller skips that step instead of panicking (#1368).

use regex::Regex;

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

pub(crate) fn compile(pattern: &'static str) -> Option<Regex> {
    match Regex::new(pattern) {
        Ok(re) => Some(re),
        Err(e) => {
            log::error!("botdocs: pattern failed to compile ({pattern}): {e}");
            None
        }
    }
}
