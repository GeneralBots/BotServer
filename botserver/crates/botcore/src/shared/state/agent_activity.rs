use super::*;

impl AgentActivity {
    pub fn new(phase: impl Into<String>) -> Self {
        Self {
            phase: phase.into(),
            items_processed: 0,
            items_total: None,
            speed_per_min: None,
            eta_seconds: None,
            current_item: None,
            bytes_processed: None,
            tokens_used: None,
            files_created: None,
            tables_created: None,
            log_lines: None,
        }
    }

    #[must_use]
    pub fn with_progress(mut self, processed: u32, total: Option<u32>) -> Self {
        self.items_processed = processed;
        self.items_total = total;
        self
    }

    #[must_use]
    pub fn with_speed(mut self, speed: f32, eta_seconds: Option<u32>) -> Self {
        self.speed_per_min = Some(speed);
        self.eta_seconds = eta_seconds;
        self
    }

    #[must_use]
    pub fn with_current_item(mut self, item: impl Into<String>) -> Self {
        self.current_item = Some(item.into());
        self
    }

    #[must_use]
    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes_processed = Some(bytes);
        self
    }

    #[must_use]
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }

    #[must_use]
    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files_created = Some(files);
        self
    }

    #[must_use]
    pub fn with_tables(mut self, tables: Vec<String>) -> Self {
        self.tables_created = Some(tables);
        self
    }

    #[must_use]
    pub fn with_log_lines(mut self, lines: Vec<String>) -> Self {
        self.log_lines = Some(lines);
        self
    }

    #[must_use]
    pub fn add_log_line(mut self, line: impl Into<String>) -> Self {
        let lines = self.log_lines.get_or_insert_with(Vec::new);
        lines.push(line.into());
        self
    }
}
