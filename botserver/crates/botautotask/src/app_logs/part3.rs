use super::*;

impl AppLogStore {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(HashMap::new()),
            global_logs: RwLock::new(VecDeque::with_capacity(MAX_LOGS_PER_APP)),
        }
    }

    pub fn log(
        &self,
        app_name: &str,
        level: LogLevel,
        source: LogSource,
        message: &str,
        details: Option<String>,
        ids: (Option<Uuid>, Option<Uuid>),
    ) {
        let entry = AppLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            source,
            message: message.to_string(),
            details,
            bot_id: ids.0,
            user_id: ids.1,
            app_name: app_name.to_string(),
            file_path: None,
            line_number: None,
            stack_trace: None,
        };
        self.add_entry(entry);
        match level {
            LogLevel::Debug => debug!("[{}] {}: {}", app_name, source, message),
            LogLevel::Info => info!("[{}] {}: {}", app_name, source, message),
            LogLevel::Warn => warn!("[{}] {}: {}", app_name, source, message),
            LogLevel::Error | LogLevel::Critical => log::error!("[{}] {}: {}", app_name, source, message),
        }
    }

    pub fn log_error(
        &self,
        app_name: &str,
        source: LogSource,
        message: &str,
        err: &str,
        location: (Option<&str>, Option<u32>, Option<&str>),
    ) {
        let entry = AppLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level: LogLevel::Error,
            source,
            app_name: app_name.to_string(),
            bot_id: None,
            user_id: None,
            message: message.to_string(),
            details: Some(err.to_string()),
            file_path: location.0.map(String::from),
            line_number: location.1,
            stack_trace: location.2.map(String::from),
        };
        self.add_entry(entry);
        error!(
            "[{}] {}: {} - {} ({}:{})",
            app_name, source, message, err,
            location.0.unwrap_or("unknown"),
            location.1.unwrap_or(0)
        );
    }

    pub fn log_client(
        &self,
        request: ClientLogRequest,
        bot_id: Option<Uuid>,
        user_id: Option<Uuid>,
    ) {
        let level = match request.level.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "warn" | "warning" => LogLevel::Warn,
            "error" => LogLevel::Error,
            "critical" => LogLevel::Critical,
            _ => LogLevel::Info,
        };
        let entry = AppLogEntry {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            level,
            source: LogSource::Client,
            app_name: request.app_name,
            bot_id,
            user_id,
            message: request.message,
            details: request.details,
            file_path: request.file_path,
            line_number: request.line_number,
            stack_trace: request.stack_trace,
        };
        match level {
            LogLevel::Debug => debug!("[{}] {}: {}", entry.app_name, entry.source, entry.message),
            LogLevel::Info => info!("[{}] {}: {}", entry.app_name, entry.source, entry.message),
            LogLevel::Warn => warn!("[{}] {}: {}", entry.app_name, entry.source, entry.message),
            LogLevel::Error | LogLevel::Critical => {
                error!("[{}] {}: {}", entry.app_name, entry.source, entry.message)
            }
        }
        self.add_entry(entry);
    }

    pub(super) fn add_entry(&self, entry: AppLogEntry) {
        if let Ok(mut logs) = self.logs.write() {
            let app_logs = logs
                .entry(entry.app_name.clone())
                .or_insert_with(|| VecDeque::with_capacity(MAX_LOGS_PER_APP));
            if app_logs.len() >= MAX_LOGS_PER_APP {
                app_logs.pop_front();
            }
            app_logs.push_back(entry.clone());
        }
        if let Ok(mut global) = self.global_logs.write() {
            if global.len() >= MAX_LOGS_PER_APP {
                global.pop_front();
            }
            global.push_back(entry);
        }
    }

    pub fn get_logs(&self, params: &LogQueryParams) -> Vec<AppLogEntry> {
        let limit = params.limit.unwrap_or(100).min(500);
        let cutoff = params
            .since
            .unwrap_or_else(|| Utc::now() - Duration::days(LOG_RETENTION_DAYS));

        let level_filter = params.level.as_ref().and_then(|l| match l.to_lowercase().as_str() {
            "debug" => Some(LogLevel::Debug),
            "info" => Some(LogLevel::Info),
            "warn" => Some(LogLevel::Warn),
            "error" => Some(LogLevel::Error),
            "critical" => Some(LogLevel::Critical),
            _ => None,
        });

        let source_filter = params.source.as_ref().and_then(|s| match s.to_lowercase().as_str() {
            "server" => Some(LogSource::Server),
            "client" => Some(LogSource::Client),
            "generator" => Some(LogSource::Generator),
            "designer" => Some(LogSource::Designer),
            "validation" => Some(LogSource::Validation),
            "runtime" => Some(LogSource::Runtime),
            _ => None,
        });

        if let Some(ref app_name) = params.app_name {
            if let Ok(logs) = self.logs.read() {
                if let Some(app_logs) = logs.get(app_name) {
                    return app_logs
                        .iter()
                        .rev()
                        .filter(|e| e.timestamp >= cutoff)
                        .filter(|e| level_filter.is_none_or(|l| e.level == l))
                        .filter(|e| source_filter.is_none_or(|s| e.source == s))
                        .take(limit)
                        .cloned()
                        .collect();
                }
            }
            return Vec::new();
        }

        if let Ok(global) = self.global_logs.read() {
            return global
                .iter()
                .rev()
                .filter(|e| e.timestamp >= cutoff)
                .filter(|e| level_filter.is_none_or(|l| e.level == l))
                .filter(|e| source_filter.is_none_or(|s| e.source == s))
                .take(limit)
                .cloned()
                .collect();
        }

        Vec::new()
    }

    pub fn get_errors_for_designer(&self, app_name: &str) -> Vec<AppLogEntry> {
        if let Ok(logs) = self.logs.read() {
            if let Some(app_logs) = logs.get(app_name) {
                let cutoff = Utc::now() - Duration::hours(1);
                return app_logs
                    .iter()
                    .rev()
                    .filter(|e| e.timestamp >= cutoff)
                    .filter(|e| matches!(e.level, LogLevel::Error | LogLevel::Critical | LogLevel::Warn))
                    .take(MAX_LOGS_FOR_DESIGNER)
                    .cloned()
                    .collect();
            }
        }
        Vec::new()
    }

    pub fn format_errors_for_prompt(&self, app_name: &str) -> Option<String> {
        let errors = self.get_errors_for_designer(app_name);
        if errors.is_empty() {
            return None;
        }
        let mut output = String::new();
        output.push_str("\n\n=== RECENT ERRORS AND WARNINGS ===\n");
        output.push_str("The following issues were detected. Please fix them:\n\n");
        for (idx, entry) in errors.iter().enumerate() {
            use std::fmt::Write;
            let _ = writeln!(output, "{}. [{}] [{}] {}", idx + 1, entry.level, entry.source, entry.message);
            if let Some(ref details) = entry.details {
                let _ = writeln!(output, "  Details: {details}");
            }
            if let Some(ref file) = entry.file_path {
                let _ = writeln!(output, "  Location: {}:{}", file, entry.line_number.unwrap_or(0));
            }
            if let Some(ref stack) = entry.stack_trace {
                let short_stack: String = stack.lines().take(3).collect::<Vec<_>>().join("\n    ");
                let _ = writeln!(output, "  Stack: {short_stack}");
            }
            output.push('\n');
        }
        output.push_str("=== END OF ERRORS ===\n");
        Some(output)
    }

    pub fn get_stats(&self) -> LogStats {
        let mut stats = LogStats {
            total_logs: 0,
            errors: 0,
            warnings: 0,
            by_app: HashMap::new(),
        };
        if let Ok(logs) = self.logs.read() {
            for (app_name, app_logs) in logs.iter() {
                let count = app_logs.len();
                stats.total_logs += count;
                stats.by_app.insert(app_name.clone(), count);
                for entry in app_logs {
                    match entry.level {
                        LogLevel::Error | LogLevel::Critical => stats.errors += 1,
                        LogLevel::Warn => stats.warnings += 1,
                        _ => {}
                    }
                }
            }
        }
        stats
    }

    pub fn cleanup_old_logs(&self) {
        let cutoff = Utc::now() - Duration::days(LOG_RETENTION_DAYS);
        if let Ok(mut logs) = self.logs.write() {
            for app_logs in logs.values_mut() {
                while let Some(front) = app_logs.front() {
                    if front.timestamp < cutoff {
                        app_logs.pop_front();
                    } else {
                        break;
                    }
                }
            }
            logs.retain(|_, v| !v.is_empty());
        }
        if let Ok(mut global) = self.global_logs.write() {
            while let Some(front) = global.front() {
                if front.timestamp < cutoff {
                    global.pop_front();
                } else {
                    break;
                }
            }
        }
        info!("Log cleanup completed");
    }

    pub fn clear_app_logs(&self, app_name: &str) {
        if let Ok(mut logs) = self.logs.write() {
            logs.remove(app_name);
        }
        info!("Cleared logs for app: {}", app_name);
    }
}
impl Default for AppLogStore {
    fn default() -> Self {
        Self::new()
    }
}
