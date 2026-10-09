use super::*;

pub(super) fn determine_metric_from_request(request: &Request<Body>) -> UsageMetric {
    let path = request.uri().path();
    let method = request.method();

    if path.contains("/chat") || path.contains("/message") || path.contains("/conversation") {
        return UsageMetric::Messages;
    }

    if path.contains("/upload") || path.contains("/file") || path.contains("/storage") {
        return UsageMetric::StorageBytes;
    }

    if path.contains("/bot") && method == "POST" {
        return UsageMetric::Bots;
    }

    if path.contains("/user") && method == "POST" {
        return UsageMetric::Users;
    }

    if path.contains("/kb") || path.contains("/document") {
        return UsageMetric::KbDocuments;
    }

    if path.contains("/app") || path.contains("/form") || path.contains("/site") {
        return UsageMetric::Apps;
    }

    UsageMetric::ApiCalls
}
