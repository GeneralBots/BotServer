use super::*;

// ---------------------------------------------------------------------------
// Reqwest client-side tracing layer
// ---------------------------------------------------------------------------

/// A helper for injecting W3C trace context into outgoing reqwest requests.
pub struct ReqwestTracingLayer;

impl ReqwestTracingLayer {
    pub fn new() -> Self {
        Self
    }

    /// Inject W3C traceparent header into a reqwest::Request before sending.
    pub fn inject_into_request(request: &mut reqwest::Request, ctx: &SpanContext) {
        let traceparent = ctx.to_w3c_traceparent();
        if let Ok(val) = reqwest::header::HeaderValue::from_str(&traceparent) {
            request.headers_mut().insert(
                reqwest::header::HeaderName::from_static("traceparent"),
                val,
            );
        }
    }

    /// Create a reqwest Client with default configuration.
    pub fn client_with_tracing() -> reqwest::Client {
        reqwest::Client::builder()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    }
}

impl Default for ReqwestTracingLayer {
    fn default() -> Self {
        Self::new()
    }
}
