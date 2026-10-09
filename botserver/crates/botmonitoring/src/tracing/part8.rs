use super::*;

impl SpanBuilder {
    pub fn new(operation_name: &str) -> Self {
        Self {
            operation_name: operation_name.to_string(),
            service_name: "botserver".to_string(),
            kind: SpanKind::Internal,
            parent_context: None,
            attributes: HashMap::new(),
            links: Vec::new(),
            start_time: None,
        }
    }

    pub fn with_service_name(mut self, name: &str) -> Self {
        self.service_name = name.to_string();
        self
    }

    pub fn with_kind(mut self, kind: SpanKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_parent(mut self, context: TraceContext) -> Self {
        self.parent_context = Some(context);
        self
    }

    pub fn with_attribute<V: Into<AttributeValue>>(mut self, key: &str, value: V) -> Self {
        self.attributes.insert(key.to_string(), value.into());
        self
    }

    pub fn with_link(mut self, trace_id: &str, span_id: &str) -> Self {
        self.links.push(SpanLink {
            trace_id: trace_id.to_string(),
            span_id: span_id.to_string(),
            attributes: HashMap::new(),
        });
        self
    }

    pub fn with_start_time(mut self, time: DateTime<Utc>) -> Self {
        self.start_time = Some(time);
        self
    }

    pub fn start(self, tracer: &DistributedTracingService) -> ActiveSpan {
        let context = match self.parent_context {
            Some(parent) => parent.child(),
            None => TraceContext::new(),
        };

        let start_time = self.start_time.unwrap_or_else(Utc::now);

        let span = Span {
            trace_id: context.trace_id.clone(),
            span_id: context.span_id.clone(),
            parent_span_id: context.parent_span_id.clone(),
            operation_name: self.operation_name,
            service_name: self.service_name,
            kind: self.kind,
            start_time,
            end_time: None,
            duration_us: None,
            status: SpanStatus::Unset,
            status_message: None,
            attributes: self.attributes,
            events: Vec::new(),
            links: self.links,
            resource: ResourceAttributes::default(),
        };

        ActiveSpan {
            span,
            context,
            tracer: tracer.clone(),
        }
    }
}
