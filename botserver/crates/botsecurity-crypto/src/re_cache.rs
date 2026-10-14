//! Cached `Regex` compilation (#1493).
//!
//! The masking rules in this crate are constant patterns assembled once at
//! start, so each one is parsed and compiled once per process instead of once
//! per call. Following `botconnectors::redact`, a pattern that fails to compile
//! (impossible for the literals used here) is logged and reported as `None`; the
//! caller skips that step instead of panicking inside the running server (#1368).

use regex::Regex;

/// Compile a constant pattern once per call site.
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
            tracing::error!(
                "{}: masking pattern failed to compile ({pattern}): {e}",
                module_path!()
            );
            None
        }
    }
}
