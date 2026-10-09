use super::*;

impl ActiveSpan {
    pub fn context(&self) -> &TraceContext {
        &self.context
    }

    pub fn set_attribute<V: Into<AttributeValue>>(&mut self, key: &str, value: V) {
        self.span.attributes.insert(key.to_string(), value.into());
    }

    pub fn add_event(&mut self, name: &str) {
        self.span.events.push(SpanEvent {
            name: name.to_string(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        });
    }

    pub fn add_event_with_attributes(&mut self, name: &str, attributes: HashMap<String, AttributeValue>) {
        self.span.events.push(SpanEvent {
            name: name.to_string(),
            timestamp: Utc::now(),
            attributes,
        });
    }

    pub fn set_status(&mut self, status: SpanStatus, message: Option<&str>) {
        self.span.status = status;
        self.span.status_message = message.map(|s| s.to_string());
    }

    pub fn record_error(&mut self, error: &str) {
        self.span.status = SpanStatus::Error;
        self.span.status_message = Some(error.to_string());

        let mut attrs = HashMap::new();
        attrs.insert("exception.message".to_string(), AttributeValue::String(error.to_string()));
        attrs.insert("exception.type".to_string(), AttributeValue::String("Error".to_string()));

        self.span.events.push(SpanEvent {
            name: "exception".to_string(),
            timestamp: Utc::now(),
            attributes: attrs,
        });
    }

    pub async fn end(mut self) {
        let end_time = Utc::now();
        self.span.end_time = Some(end_time);
        self.span.duration_us = Some((end_time - self.span.start_time).num_microseconds().unwrap_or(0));

        if self.span.status == SpanStatus::Unset {
            self.span.status = SpanStatus::Ok;
        }

        self.tracer.record_span(self.span).await;
    }
}
