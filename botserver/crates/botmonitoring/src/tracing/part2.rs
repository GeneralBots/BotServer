use super::*;

impl TraceContext {
    pub fn new() -> Self {
        Self {
            trace_id: generate_trace_id(),
            span_id: generate_span_id(),
            parent_span_id: None,
            trace_flags: 1,
            trace_state: HashMap::new(),
            baggage: HashMap::new(),
        }
    }

    pub fn child(&self) -> Self {
        Self {
            trace_id: self.trace_id.clone(),
            span_id: generate_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            trace_flags: self.trace_flags,
            trace_state: self.trace_state.clone(),
            baggage: self.baggage.clone(),
        }
    }

    pub fn to_w3c_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id, self.span_id, self.trace_flags
        )
    }

    pub fn from_w3c_traceparent(header: &str) -> Option<Self> {
        let parts: Vec<&str> = header.split('-').collect();
        if parts.len() != 4 {
            return None;
        }

        let trace_flags = u8::from_str_radix(parts[3], 16).ok()?;

        Some(Self {
            trace_id: parts[1].to_string(),
            span_id: parts[2].to_string(),
            parent_span_id: None,
            trace_flags,
            trace_state: HashMap::new(),
            baggage: HashMap::new(),
        })
    }

    pub fn to_b3_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("X-B3-TraceId".to_string(), self.trace_id.clone());
        headers.insert("X-B3-SpanId".to_string(), self.span_id.clone());
        if let Some(parent) = &self.parent_span_id {
            headers.insert("X-B3-ParentSpanId".to_string(), parent.clone());
        }
        headers.insert(
            "X-B3-Sampled".to_string(),
            if self.trace_flags & 1 == 1 { "1" } else { "0" }.to_string(),
        );
        headers
    }
}
impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}
