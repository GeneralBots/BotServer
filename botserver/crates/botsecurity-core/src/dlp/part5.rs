use super::*;

impl DlpMatch {
    pub fn new(
        data_type: SensitiveDataType,
        matched_text: &str,
        start: usize,
        end: usize,
    ) -> Self {
        Self {
            data_type,
            matched_text: matched_text.to_string(),
            start_position: start,
            end_position: end,
            confidence: 1.0,
            context: None,
        }
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }

    pub fn with_context(mut self, context: String) -> Self {
        self.context = Some(context);
        self
    }

    pub fn masked_value(&self) -> String {
        let len = self.matched_text.len();
        if len <= 4 {
            "*".repeat(len)
        } else {
            format!(
                "{}{}{}",
                &self.matched_text[..2],
                "*".repeat(len - 4),
                &self.matched_text[len - 2..]
            )
        }
    }
}
