
/// Convenience detection for the API-call trigger. Some models emit the JSON
/// key as `__api_call__` (canonical) or `api_call` (common off-by-underscore
/// variant) or `_api_call_`. Tolerate all so the trigger is never missed and
/// never rendered to the user as raw JSON.
pub fn is_api_call_trigger(hay: &str) -> bool {
    ["\"__api_call__\":", "\"api_call\":", "\"_api_call_\":", "\"api-call\":"]
        .iter()
        .any(|t| hay.contains(t))
}
/// Position of the first API-call trigger variant in `hay`, if any.
pub fn find_api_call(hay: &str) -> Option<usize> {
    ["\"__api_call__\":", "\"api_call\":", "\"_api_call_\":", "\"api-call\":"]
        .iter()
        .filter_map(|t| hay.find(t))
        .min()
}
