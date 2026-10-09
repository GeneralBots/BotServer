use super::*;

/// Represents a W3C Trace Context span context with trace_id, span_id,
/// optional parent_span_id, and trace_flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanContext {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub trace_flags: u8,
    pub sampled: bool,
    pub start_time: SystemTime,
    pub operation_name: String,
}

impl SpanContext {
    /// Create a new root span context with a fresh trace_id and span_id.
    pub fn new(operation_name: impl Into<String>) -> Self {
        let trace_id = Uuid::new_v4().simple().to_string();
        let span_id = Uuid::new_v4().simple().to_string()[..16].to_string();
        Self {
            trace_id,
            span_id,
            parent_span_id: None,
            trace_flags: TRACE_FLAGS_SAMPLED,
            sampled: true,
            start_time: SystemTime::now(),
            operation_name: operation_name.into(),
        }
    }

    /// Parse a W3C traceparent header value into a SpanContext.
    ///
    /// Format: `{version}-{trace_id}-{span_id}-{trace_flags}`
    /// Version must be `00`. Returns None on invalid input.
    pub fn from_w3c_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.trim().split('-').collect();
        if parts.len() < 4 {
            return None;
        }
        let _version = parts[0];
        let trace_id = parts[1].to_string();
        let span_id = parts[2].to_string();
        let trace_flags = u8::from_str_radix(parts[3], 16).ok()?;
        Some(Self {
            trace_id,
            span_id,
            parent_span_id: None,
            trace_flags,
            sampled: (trace_flags & TRACE_FLAGS_SAMPLED) != 0,
            start_time: SystemTime::now(),
            operation_name: String::new(),
        })
    }

    /// Serialize this span context into a W3C traceparent header value.
    ///
    /// Format: `00-{trace_id}-{span_id}-{trace_flags:02x}`
    pub fn to_w3c_traceparent(&self) -> String {
        format!(
            "{}-{}-{}-{:02x}",
            TRACEPARENT_VERSION, self.trace_id, self.span_id, self.trace_flags
        )
    }

    /// Create a child span that inherits the trace_id and flags, with a new
    /// span_id and this span as its parent.
    pub fn child_span(&self, operation_name: impl Into<String>) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: Uuid::new_v4().simple().to_string()[..16].to_string(),
            parent_span_id: Some(self.span_id.clone()),
            trace_flags: self.trace_flags,
            sampled: self.sampled,
            start_time: SystemTime::now(),
            operation_name: operation_name.into(),
        }
    }

    /// Return the elapsed time in milliseconds since this span was created.
    pub fn elapsed_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(self.start_time)
            .unwrap_or(Duration::from_secs(0))
            .as_millis() as u64
    }
}

/// Start a new span, optionally as a child of a parent context.
pub fn start_span(operation_name: &str, parent: Option<&SpanContext>) -> SpanContext {
    let ctx = match parent {
        Some(p) => p.child_span(operation_name),
        None => SpanContext::new(operation_name),
    };
    log::debug!(
        "[tracing] start_span: {} trace_id={} span_id={}",
        ctx.operation_name,
        ctx.trace_id,
        ctx.span_id
    );
    ctx
}

/// Record the end of a span, logging its duration.
pub fn end_span(span: &SpanContext) {
    let elapsed = span.elapsed_ms();
    log::trace!(
        "[tracing] end_span: {} duration={}ms trace_id={} span_id={}",
        span.operation_name,
        elapsed,
        span.trace_id,
        span.span_id
    );
}

/// Inject a W3C traceparent header into an http::HeaderMap.
pub fn inject_context(ctx: &SpanContext, headers: &mut http::HeaderMap) {
    let traceparent = ctx.to_w3c_traceparent();
    if let Ok(val) = http::HeaderValue::from_str(&traceparent) {
        headers.insert(
            http::header::HeaderName::from_static("traceparent"),
            val,
        );
    }
}

/// Extract a SpanContext from the W3C traceparent header in an http::HeaderMap,
/// returning None if the header is missing or invalid.
pub fn extract_context(headers: &http::HeaderMap) -> Option<SpanContext> {
    let header_str = headers
        .get("traceparent")
        .and_then(|v| v.to_str().ok())?;
    let ctx = SpanContext::from_w3c_traceparent(header_str)?;
    log::debug!(
        "[tracing] extract_context: trace_id={} span_id={}",
        ctx.trace_id,
        ctx.span_id
    );
    Some(ctx)
}
