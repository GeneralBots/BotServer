// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------


#[test]
pub(super) fn test_traceparent_roundtrip() {
    let ctx = SpanContext::new("test-operation");
    let traceparent = ctx.to_w3c_traceparent();
    let parsed = SpanContext::from_w3c_traceparent(&traceparent);
    assert!(parsed.is_some());
    let parsed = parsed.unwrap();
    assert_eq!(ctx.trace_id, parsed.trace_id);
    assert_eq!(ctx.span_id, parsed.span_id);
    assert_eq!(ctx.trace_flags, parsed.trace_flags);
}

#[test]
pub(super) fn test_child_span() {
    let parent = SpanContext::new("parent");
    let child = parent.child_span("child");
    assert_eq!(parent.trace_id, child.trace_id);
    assert_ne!(parent.span_id, child.span_id);
    assert_eq!(child.parent_span_id, Some(parent.span_id));
}

#[test]
pub(super) fn test_traceparent_format() {
    let ctx = SpanContext::new("test");
    let tp = ctx.to_w3c_traceparent();
    assert!(tp.starts_with("00-"));
    // Format: 00-{32 hex chars}-{16 hex chars}-{2 hex chars}
    assert_eq!(tp.len(), 55);
}

#[test]
pub(super) fn test_invalid_traceparent() {
    assert!(SpanContext::from_w3c_traceparent("invalid").is_none());
    assert!(SpanContext::from_w3c_traceparent("").is_none());
    assert!(SpanContext::from_w3c_traceparent("00-abc-123-01").is_some());
}

#[test]
pub(super) fn test_span_guard_auto_finish() {
    let mut guard = SpanGuard::new("test", None);
    assert_eq!(guard.context().operation_name, "test");
    guard.finish();
}

#[test]
pub(super) fn test_extract_context_no_header() {
    let headers = http::HeaderMap::new();
    assert!(extract_context(&headers).is_none());
}

#[test]
pub(super) fn test_extract_context_with_header() {
    let mut headers = http::HeaderMap::new();
    let ctx = SpanContext::new("test");
    let tp = ctx.to_w3c_traceparent();
    headers.insert(
        "traceparent",
        http::HeaderValue::from_str(&tp).ok().unwrap(),
    );
    let extracted = extract_context(&headers);
    assert!(extracted.is_some());
    let extracted = extracted.unwrap();
    assert_eq!(ctx.trace_id, extracted.trace_id);
    assert_eq!(ctx.span_id, extracted.span_id);
}

#[test]
pub(super) fn test_trace_id_generation_unique() {
    let ctx1 = SpanContext::new("a");
    let ctx2 = SpanContext::new("b");
    assert_ne!(ctx1.trace_id, ctx2.trace_id);
}

#[test]
pub(super) fn test_span_id_generation_unique() {
    let ctx1 = SpanContext::new("a");
    let ctx2 = SpanContext::new("b");
    assert_ne!(ctx1.span_id, ctx2.span_id);
}

#[test]
pub(super) fn test_inject_context() {
    let ctx = SpanContext::new("test");
    let mut headers = http::HeaderMap::new();
    inject_context(&ctx, &mut headers);
    assert!(headers.contains_key("traceparent"));
    let tp = headers.get("traceparent").unwrap().to_str().unwrap();
    assert!(tp.starts_with("00-"));
}

#[test]
pub(super) fn test_tower_layer_creation() {
    let layer = TracingMiddleware::new("test-service");
    assert_eq!(layer.service_name, "test-service");
}

#[test]
pub(super) fn test_reqwest_layer_default() {
    let _layer = ReqwestTracingLayer::new();
    let _client = ReqwestTracingLayer::client_with_tracing();
}

#[test]
pub(super) fn test_tracestate_header() {
    assert_eq!(tracestate_header(), "tracestate");
    assert_eq!(traceparent_header(), "traceparent");
}

#[test]
pub(super) fn test_parent_span_id_none_for_root() {
    let ctx = SpanContext::new("root");
    assert!(ctx.parent_span_id.is_none());
}

#[test]
pub(super) fn test_child_preserves_trace_flags() {
    let parent = SpanContext::new("parent");
    let child = parent.child_span("child");
    assert_eq!(parent.trace_flags, child.trace_flags);
    assert_eq!(parent.sampled, child.sampled);
}

#[test]
pub(super) fn test_elapsed_ms_non_negative() {
    let ctx = SpanContext::new("timer");
    assert!(ctx.elapsed_ms() < 1000);
}
