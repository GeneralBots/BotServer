use super::*;

pub fn log_generator_info(app_name: &str, message: &str) {
    APP_LOGS.log(app_name, LogLevel::Info, LogSource::Generator, message, None, (None, None));
}
pub fn log_generator_error(app_name: &str, message: &str, error: &str) {
    APP_LOGS.log_error(app_name, LogSource::Generator, message, error, (None, None, None));
}
pub fn log_validation_error(app_name: &str, message: &str, file_path: Option<&str>, line_number: Option<u32>) {
    APP_LOGS.log_error(app_name, LogSource::Validation, message, "Validation failed", (file_path, line_number, None));
}
pub fn log_runtime_error(app_name: &str, message: &str, error: &str, stack_trace: Option<&str>) {
    APP_LOGS.log_error(app_name, LogSource::Runtime, message, error, (None, None, stack_trace));
}
pub fn get_designer_error_context(app_name: &str) -> Option<String> {
    APP_LOGS.format_errors_for_prompt(app_name)
}
pub fn start_log_cleanup_scheduler() {
    std::thread::spawn(|| loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
        APP_LOGS.cleanup_old_logs();
    });
    info!("Log cleanup scheduler started (runs hourly)");
}
pub fn generate_client_logger_js() -> &'static str {
    r#"
(function() {
  const APP_NAME = document.body.dataset.appName || window.location.pathname.split('/')[1] || 'unknown';
  const LOG_ENDPOINT = '/api/app-logs/client';
  const LOG_BUFFER = [];
  const FLUSH_INTERVAL = 5000;
  const MAX_BUFFER_SIZE = 50;

  function sendLogs() {
    if (LOG_BUFFER.length === 0) return;
    const logs = LOG_BUFFER.splice(0, LOG_BUFFER.length);
    fetch(LOG_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ logs: logs })
    }).catch(function(e) { console.warn('Failed to send logs:', e); });
  }

  function addLog(level, message, details) {
    const entry = {
      app_name: APP_NAME, level: level, message: message,
      details: details || null, file_path: null, line_number: null,
      stack_trace: null, user_agent: navigator.userAgent
    };
    LOG_BUFFER.push(entry);
    if (LOG_BUFFER.length >= MAX_BUFFER_SIZE) { sendLogs(); }
  }

  window.onerror = function(message, source, lineno, colno, error) {
    addLog('error', message, JSON.stringify({ source: source, line: lineno, column: colno, stack: error ? error.stack : null }));
    return false;
  };

  window.onunhandledrejection = function(event) {
    addLog('error', 'Unhandled Promise Rejection: ' + event.reason, event.reason && event.reason.stack ? event.reason.stack : null);
  };

  const originalConsoleError = console.error;
  console.error = function() { addLog('error', Array.from(arguments).join(' ')); originalConsoleError.apply(console, arguments); };

  const originalConsoleWarn = console.warn;
  console.warn = function() { addLog('warn', Array.from(arguments).join(' ')); originalConsoleWarn.apply(console, arguments); };

  document.body.addEventListener('htmx:responseError', function(evt) {
    addLog('error', 'HTMX Request Failed', JSON.stringify({ url: evt.detail.xhr.responseURL, status: evt.detail.xhr.status, response: evt.detail.xhr.responseText.substring(0, 500) }));
  });

  document.body.addEventListener('htmx:sendError', function(evt) {
    addLog('error', 'HTMX Send Error', JSON.stringify({ url: evt.detail.requestConfig.path }));
  });

  setInterval(sendLogs, FLUSH_INTERVAL);
  window.addEventListener('beforeunload', sendLogs);

  window.AppLogger = {
    debug: function(msg, details) { addLog('debug', msg, details); },
    info: function(msg, details) { addLog('info', msg, details); },
    warn: function(msg, details) { addLog('warn', msg, details); },
    error: function(msg, details) { addLog('error', msg, details); },
    flush: sendLogs
  };

  console.log('[AppLogger] Initialized for app:', APP_NAME);
})();
"#
}
