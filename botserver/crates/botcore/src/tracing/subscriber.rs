use super::*;

// ---------------------------------------------------------------------------
// Subscriber setup
// ---------------------------------------------------------------------------

/// Initialize the tracing subscriber with sensible defaults.
///
/// Safe to call multiple times — subsequent calls are silently ignored.
pub fn setup_tracing_subscriber() {
    let _ = tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .try_init();
}

// ---------------------------------------------------------------------------
// Header name constants
// ---------------------------------------------------------------------------

/// Axum middleware function used by `axum::middleware::from_fn` to attach
/// distributed-tracing headers and log the request span. Kept as a free
/// function so it can be re-used across routers without taking ownership of
/// the layer.
pub async fn tracing_middleware_fn(
    name: String,
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let parent = extract_context(req.headers());
    let span = start_span(&name, parent.as_ref());
    let method = req.method().clone();
    let uri = req.uri().clone();
    log::info!(
        "tracing.span.start name={} method={} uri={} trace_id={}",
        name,
        method,
        uri,
        span.trace_id
    );
    let mut response = next.run(req).await;
    if let Some(ctx) = parent.as_ref() {
        inject_context(ctx, response.headers_mut());
    }
    log::info!(
        "tracing.span.end name={} trace_id={} status={}",
        name,
        span.trace_id,
        response.status()
    );
    response
}

pub fn traceparent_header() -> &'static str {
    "traceparent"
}

pub fn tracestate_header() -> &'static str {
    "tracestate"
}

// ---------------------------------------------------------------------------
// RAII SpanGuard
// ---------------------------------------------------------------------------

/// A guard that automatically ends a span when dropped (RAII pattern).
pub struct SpanGuard {
    ctx: SpanContext,
    finished: bool,
}

impl SpanGuard {
    pub fn new(operation_name: impl Into<String>, parent: Option<&SpanContext>) -> Self {
        Self {
            ctx: start_span(&operation_name.into(), parent),
            finished: false,
        }
    }

    pub fn context(&self) -> &SpanContext {
        &self.ctx
    }

    pub fn child(&self, operation_name: impl Into<String>) -> Self {
        Self::new(operation_name, Some(&self.ctx))
    }

    pub fn finish(&mut self) {
        if !self.finished {
            end_span(&self.ctx);
            self.finished = true;
        }
    }
}

impl Drop for SpanGuard {
    fn drop(&mut self) {
        self.finish();
    }
}
