use super::*;

impl ApprovalManager {
    pub fn new(config: ApprovalConfig) -> Self {
        Self { config }
    }

    pub fn from_config(config_map: &HashMap<String, String>) -> Self {
        let config = ApprovalConfig {
            enabled: config_map
                .get("approval-enabled")
                .map(|v| v == "true")
                .unwrap_or(true),
            default_timeout: config_map
                .get("approval-default-timeout")
                .and_then(|v| v.parse().ok())
                .unwrap_or(3600),
            reminder_interval: config_map
                .get("approval-reminder-interval")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1800),
            max_reminders: config_map
                .get("approval-max-reminders")
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            audit_enabled: config_map
                .get("approval-audit-enabled")
                .map(|v| v == "true")
                .unwrap_or(true),
            webhook_url: config_map.get("approval-webhook-url").cloned(),
            email_template: config_map.get("approval-email-template").cloned(),
            approval_base_url: config_map.get("approval-base-url").cloned(),
        };
        Self::new(config)
    }

    pub fn create_request(
        &self,
        params: CreateApprovalRequestParams<'_>,
    ) -> ApprovalRequest {
        let timeout = params.timeout_seconds.unwrap_or(self.config.default_timeout);
        let now = Utc::now();

        ApprovalRequest {
            id: Uuid::new_v4(),
            bot_id: params.bot_id,
            session_id: params.session_id,
            initiated_by: params.initiated_by,
            approval_type: params.approval_type.to_string(),
            status: ApprovalStatus::Pending,
            channel: params.channel,
            recipient: params.recipient.to_string(),
            context: params.context,
            message: params.message.to_string(),
            timeout_seconds: timeout,
            default_action: params.default_action,
            current_level: 1,
            total_levels: 1,
            created_at: now,
            expires_at: now + Duration::seconds(timeout as i64),
            reminders_sent: Vec::new(),
            decision: None,
            decided_by: None,
            decided_at: None,
            comments: None,
        }
    }

    pub fn is_expired(&self, request: &ApprovalRequest) -> bool {
        Utc::now() > request.expires_at
    }

    pub fn should_send_reminder(&self, request: &ApprovalRequest) -> bool {
        if request.status != ApprovalStatus::Pending {
            return false;
        }

        if request.reminders_sent.len() >= self.config.max_reminders as usize {
            return false;
        }

        let last_notification = request
            .reminders_sent
            .last()
            .copied()
            .unwrap_or(request.created_at);

        let since_last = Utc::now() - last_notification;
        since_last.num_seconds() >= self.config.reminder_interval as i64
    }

    pub fn generate_approval_url(&self, request_id: Uuid, action: &str, token: &str) -> String {
        let base_url = self
            .config
            .approval_base_url
            .as_deref()
            .unwrap_or("https://bot.example.com/approve");

        format!(
            "{}/{}?action={}&token={}",
            base_url, request_id, action, token
        )
    }

    pub fn generate_email_content(&self, request: &ApprovalRequest, token: &str) -> EmailContent {
        let approve_url = self.generate_approval_url(request.id, "approve", token);
        let reject_url = self.generate_approval_url(request.id, "reject", token);

        let subject = format!(
            "Approval Required: {} ({})",
            request.approval_type, request.id
        );

        let body = format!(
            r"
An approval is requested for:

Type: {}
Message: {}

Context:
{}

This request will expire at: {}

To approve, click: {}
To reject, click: {}

If you have questions, reply to this email.
",
            request.approval_type,
            request.message,
            serde_json::to_string_pretty(&request.context).unwrap_or_default(),
            request.expires_at.format("%Y-%m-%d %H:%M:%S UTC"),
            approve_url,
            reject_url
        );

        EmailContent {
            subject,
            body,
            html_body: None,
        }
    }

    pub fn process_decision(
        &self,
        request: &mut ApprovalRequest,
        decision: ApprovalDecision,
        decided_by: &str,
        comments: Option<String>,
    ) {
        request.decision = Some(decision.clone());
        request.decided_by = Some(decided_by.to_string());
        request.decided_at = Some(Utc::now());
        request.comments = comments;

        request.status = match decision {
            ApprovalDecision::Approve => ApprovalStatus::Approved,
            ApprovalDecision::Reject => ApprovalStatus::Rejected,
            ApprovalDecision::Escalate => ApprovalStatus::Escalated,
            ApprovalDecision::Defer | ApprovalDecision::RequestInfo => ApprovalStatus::Pending,
        };
    }

    pub fn handle_timeout(&self, request: &mut ApprovalRequest) {
        if let Some(default_action) = &request.default_action {
            request.decision = Some(default_action.clone());
            request.decided_by = Some("system:timeout".to_string());
            request.decided_at = Some(Utc::now());
            request.status = match default_action {
                ApprovalDecision::Approve => ApprovalStatus::Approved,
                ApprovalDecision::Reject => ApprovalStatus::Rejected,
                _ => ApprovalStatus::TimedOut,
            };
        } else {
            request.status = ApprovalStatus::TimedOut;
        }
    }

    pub fn evaluate_condition(
        &self,
        condition: &str,
        context: &serde_json::Value,
    ) -> Result<bool, String> {
        let parts: Vec<&str> = condition.split_whitespace().collect();
        if parts.len() != 3 {
            return Err(format!("Invalid condition format: {}", condition));
        }

        let field = parts[0];
        let operator = parts[1];
        let value_str = parts[2];

        let field_value = context
            .get(field)
            .and_then(|v| v.as_f64())
            .ok_or_else(|| format!("Field not found or not numeric: {}", field))?;

        let compare_value: f64 = value_str
            .parse()
            .map_err(|_| format!("Invalid comparison value: {}", value_str))?;

        let result = match operator {
            ">" => field_value > compare_value,
            ">=" => field_value >= compare_value,
            "<" => field_value < compare_value,
            "<=" => field_value <= compare_value,
            "==" | "=" => (field_value - compare_value).abs() < f64::EPSILON,
            "!=" => (field_value - compare_value).abs() >= f64::EPSILON,
            _ => return Err(format!("Unknown operator: {}", operator)),
        };

        Ok(result)
    }
}
