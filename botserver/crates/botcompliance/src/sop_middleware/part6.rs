use super::*;

pub(super) fn categorize_request(method: &Method, path: &str) -> SopCategory {
    let path_lower = path.to_lowercase();

    if path_lower.contains("/auth") || path_lower.contains("/login") || path_lower.contains("/logout") {
        return SopCategory::Authentication;
    }

    if path_lower.contains("/users") || path_lower.contains("/members") {
        return SopCategory::UserManagement;
    }

    if path_lower.contains("/roles") || path_lower.contains("/permissions") {
        return SopCategory::RoleManagement;
    }

    if path_lower.contains("/billing") || path_lower.contains("/subscription") || path_lower.contains("/invoice") {
        return SopCategory::BillingOperation;
    }

    if path_lower.contains("/compliance") || path_lower.contains("/audit") {
        return SopCategory::ComplianceAction;
    }

    if path_lower.contains("/settings") || path_lower.contains("/config") {
        return SopCategory::ConfigurationChange;
    }

    if path_lower.contains("/files") || path_lower.contains("/documents") || path_lower.contains("/drive") {
        return SopCategory::FileAccess;
    }

    match *method {
        Method::GET | Method::HEAD => SopCategory::DataAccess,
        Method::POST => SopCategory::DataModification,
        Method::PUT | Method::PATCH => SopCategory::DataModification,
        Method::DELETE => SopCategory::DataDeletion,
        _ => SopCategory::ApiAccess,
    }
}
pub(super) fn determine_severity(method: &Method, path: &str, status: StatusCode) -> SopSeverity {
    if status.is_server_error() {
        return SopSeverity::High;
    }

    if status == StatusCode::FORBIDDEN || status == StatusCode::UNAUTHORIZED {
        return SopSeverity::Medium;
    }

    let path_lower = path.to_lowercase();
    if path_lower.contains("/admin") || path_lower.contains("/security") {
        return SopSeverity::Medium;
    }

    if *method == Method::DELETE {
        return SopSeverity::Medium;
    }

    if (path_lower.contains("/config") || path_lower.contains("/settings"))
        && *method != Method::GET {
            return SopSeverity::Medium;
        }

    SopSeverity::Info
}
pub(super) fn determine_outcome(status: StatusCode) -> SopOutcome {
    if status.is_success() {
        SopOutcome::Success
    } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
        SopOutcome::Denied
    } else if status.is_server_error() {
        SopOutcome::Error
    } else if status == StatusCode::REQUEST_TIMEOUT || status == StatusCode::GATEWAY_TIMEOUT {
        SopOutcome::Timeout
    } else {
        SopOutcome::Failure
    }
}
pub async fn sop_logging_middleware(
    request: Request<Body>,
    next: Next,
) -> Response {
    let start_time = std::time::Instant::now();
    let request_id = Uuid::new_v4().to_string();

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let ip_address = request
        .headers()
        .get("x-forwarded-for")
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let user_id = request
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok());

    let organization_id = request
        .headers()
        .get("x-organization-id")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| Uuid::parse_str(s).ok());

    let response = next.run(request).await;

    let duration_ms = start_time.elapsed().as_millis() as u64;
    let status = response.status();

    let category = categorize_request(&method, &path);
    let severity = determine_severity(&method, &path, status);
    let outcome = determine_outcome(status);

    let mut entry = SopLogEntry::new(
        category,
        &format!("{}_{}", method.as_str().to_lowercase(), sanitize_path(&path)),
        &format!("{} {}", method, path),
    )
    .with_severity(severity)
    .with_outcome(outcome)
    .with_request_info(method.as_str(), &path, Some(request_id))
    .with_client_info(ip_address, user_agent)
    .with_response(status.as_u16(), duration_ms);

    if let Some(uid) = user_id {
        entry = entry.with_user(uid, None);
    }

    if let Some(oid) = organization_id {
        entry = entry.with_organization(oid);
    }

    let logger = SopLogger::new();
    logger.log(entry).await;

    response
}
pub(super) fn sanitize_path(path: &str) -> String {
    let uuid_pattern = regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}").ok();
    let numeric_pattern = regex::Regex::new(r"/\d+").ok();

    let mut sanitized = path.to_string();

    if let Some(re) = uuid_pattern {
        sanitized = re.replace_all(&sanitized, ":id").to_string();
    }

    if let Some(re) = numeric_pattern {
        sanitized = re.replace_all(&sanitized, "/{id}").to_string();
    }

    sanitized
        .replace('/', "_")
        .trim_matches('_')
        .to_lowercase()
}
