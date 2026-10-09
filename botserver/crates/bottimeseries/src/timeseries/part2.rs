use super::*;

impl MetricPoint {
    pub fn new(measurement: impl Into<String>) -> Self {
        Self {
            measurement: measurement.into(),
            tags: HashMap::new(),
            fields: HashMap::new(),
            timestamp: None,
        }
    }

    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    pub fn field_f64(mut self, key: impl Into<String>, value: f64) -> Self {
        self.fields.insert(key.into(), FieldValue::Float(value));
        self
    }

    pub fn field_i64(mut self, key: impl Into<String>, value: i64) -> Self {
        self.fields.insert(key.into(), FieldValue::Integer(value));
        self
    }

    pub fn field_u64(mut self, key: impl Into<String>, value: u64) -> Self {
        self.fields
            .insert(key.into(), FieldValue::UnsignedInteger(value));
        self
    }

    pub fn field_str(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields
            .insert(key.into(), FieldValue::String(value.into()));
        self
    }

    pub fn field_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.fields.insert(key.into(), FieldValue::Boolean(value));
        self
    }

    pub fn at(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    pub fn to_line_protocol(&self) -> String {
        let mut line = self.measurement.clone();

        let mut sorted_tags: Vec<_> = self.tags.iter().collect();
        sorted_tags.sort_by_key(|(k, _)| *k);
        for (key, value) in sorted_tags {
            line.push(',');
            line.push_str(&escape_tag_key(key));
            line.push('=');
            line.push_str(&escape_tag_value(value));
        }

        line.push(' ');
        let mut sorted_fields: Vec<_> = self.fields.iter().collect();
        sorted_fields.sort_by_key(|(k, _)| *k);
        let fields_str: Vec<String> = sorted_fields
            .iter()
            .map(|(key, value)| {
                let escaped_key = escape_field_key(key);
                match value {
                    FieldValue::Float(v) => format!("{}={}", escaped_key, v),
                    FieldValue::Integer(v) => format!("{}={}i", escaped_key, v),
                    FieldValue::UnsignedInteger(v) => format!("{}={}u", escaped_key, v),
                    FieldValue::String(v) => {
                        format!("{}=\"{}\"", escaped_key, escape_string_value(v))
                    }
                    FieldValue::Boolean(v) => format!("{}={}", escaped_key, v),
                }
            })
            .collect();
        line.push_str(&fields_str.join(","));

        if let Some(ts) = self.timestamp {
            line.push(' ');
            line.push_str(&ts.timestamp_nanos_opt().unwrap_or(0).to_string());
        }

        line
    }
}
pub(super) fn escape_tag_key(s: &str) -> String {
    s.replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}
pub(super) fn escape_tag_value(s: &str) -> String {
    s.replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}
pub(super) fn escape_field_key(s: &str) -> String {
    s.replace(',', "\\,")
        .replace('=', "\\=")
        .replace(' ', "\\ ")
}
pub(super) fn escape_string_value(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
