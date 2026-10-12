use super::*;

pub(super) fn looks_like_connection_string(text: &str) -> bool {
    let indicators = [
        "://",
        "host=",
        "dbname=",
        "user=",
        "password=",
        "port=",
        "sslmode=",
    ];

    let lower = text.to_lowercase();

    // A scheme-qualified URL is a connection string on its own: it carries the
    // host, and frequently the credentials (`postgres://user:pass@host/db`).
    // Counting indicators alone missed it, because such a string usually
    // contains only `://`.
    const CONNECTION_SCHEMES: &[&str] = &[
        "postgres://",
        "postgresql://",
        "mysql://",
        "mariadb://",
        "mongodb://",
        "mongodb+srv://",
        "redis://",
        "rediss://",
        "valkey://",
        "amqp://",
        "kafka://",
        "ldap://",
        "jdbc:",
        "ftp://",
        "sftp://",
        "http://",
        "https://",
    ];
    if CONNECTION_SCHEMES.iter().any(|scheme| lower.contains(scheme)) {
        return true;
    }

    let count = indicators.iter().filter(|i| lower.contains(*i)).count();
    count >= 2
}
pub fn sanitize_for_log(message: &str) -> String {
    let sanitizer = ErrorSanitizer::production();
    sanitizer.sanitize_message(message)
}
pub fn safe_error<E: std::error::Error>(error: E) -> SafeErrorResponse {
    let sanitizer = ErrorSanitizer::production();
    sanitizer.sanitize_error(&error, None)
}
pub fn safe_error_with_request_id<E: std::error::Error>(
    error: E,
    request_id: &str,
) -> SafeErrorResponse {
    let sanitizer = ErrorSanitizer::production();
    sanitizer.sanitize_error(&error, Some(request_id))
}
pub fn log_and_sanitize<E: std::error::Error>(
    error: &E,
    context: &str,
    request_id: Option<&str>,
) -> SafeErrorResponse {
    warn!(
        context = %context,
        request_id = ?request_id,
        error = %error,
        "Error occurred"
    );

    let sanitizer = ErrorSanitizer::production();
    sanitizer.sanitize_error(error, request_id)
}
pub fn log_and_sanitize_str(
    error_msg: &str,
    context: &str,
    request_id: Option<&str>,
) -> SafeErrorResponse {
    warn!(
        context = %context,
        request_id = ?request_id,
        error = %error_msg,
        "Error occurred"
    );

    SafeErrorResponse::internal_error()
        .with_request_id(request_id.unwrap_or("unknown").to_string())
}
