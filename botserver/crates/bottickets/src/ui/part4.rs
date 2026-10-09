use super::*;

/// `/api/ui/tickets/ai-suggest` — analyze a draft description and suggest
/// a category, priority and probable solution before the ticket is created.
pub(super) async fn handle_ai_suggest(
    axum::extract::Form(form): axum::extract::Form<AiSuggestForm>,
) -> impl IntoResponse {
    let text = format!(
        "{} {}",
        form.subject.as_deref().unwrap_or(""),
        form.description.as_deref().unwrap_or("")
    );
    let lower = text.to_lowercase();

    if text.trim().is_empty() {
        return Html(String::new());
    }

    let (category, priority, hint) = suggest_category(&lower);

    Html(format!(
        r#"<div class="ai-suggestion">
    <div class="ai-suggestion-category"><span class="ai-label">Suggested category</span><strong>{category}</strong></div>
    <div class="ai-suggestion-priority"><span class="ai-label">Suggested priority</span><strong class="priority-{prio_class}">{priority}</strong></div>
    <div class="ai-suggestion-hint"><span class="ai-label">Likely solution</span><span>{hint}</span></div>
</div>"#,
        prio_class = priority.to_lowercase()
    ))
}
pub(super) fn suggest_category(text: &str) -> (&'static str, &'static str, &'static str) {
    let rules: &[(&[&str], &str, &str, &str)] = &[
        (&["login", "password", "sign in", "authentication", "otp", "2fa"], "Authentication", "High", "Reset credentials or check the authentication service; verify token expiry and 2FA status."),
        (&["payment", "charge", "invoice", "billing", "refund"], "Billing", "High", "Verify the payment transaction and billing subscription status; check for charge failures."),
        (&["email", "not receiving", "spam", "delivery"], "Email", "Medium", "Check email delivery logs, SPF/DKIM records and the mail queue for the affected address."),
        (&["slow", "lag", "timeout", "performance"], "Performance", "High", "Inspect server load, database latency and network; review recent deploys and metrics."),
        (&["crash", "error", "exception", "bug", "fails"], "Bug", "Medium", "Collect the stack trace and reproduce steps; check recent code changes and service logs."),
        (&["install", "setup", "configure", "integration"], "Onboarding", "Low", "Follow the installation checklist; verify environment variables and service connectivity."),
        (&["virus", "phishing", "security", "breach", "malware"], "Security", "Critical", "Isolate the affected system, revoke exposed credentials and review access logs immediately."),
        (&["chat", "bot", "conversation", "suggestion"], "Chat Bot", "Medium", "Review the bot knowledge base and script logs; test the conversation flow in a staging session."),
    ];

    for (keywords, category, priority, hint) in rules {
        if keywords.iter().any(|k| text.contains(k)) {
            return (category, priority, hint);
        }
    }

    ("General", "Low", "Review the issue details and route to the appropriate team for triage.")
}
