use super::*;

impl PolicyChecker {
    pub fn new() -> Self {
        let mut checker = Self {
            policies: HashMap::new(),
            violations: Vec::new(),
            check_history: Vec::new(),
        };

        checker.initialize_default_policies();
        checker
    }

    pub(super) fn initialize_default_policies(&mut self) {
        let password_policy = SecurityPolicy {
            id: Uuid::new_v4(),
            name: "Password Strength Policy".to_string(),
            description: "Enforces strong password requirements".to_string(),
            policy_type: PolicyType::PasswordStrength,
            status: PolicyStatus::Active,
            severity: PolicySeverity::High,
            rules: vec![
                PolicyRule {
                    id: Uuid::new_v4(),
                    name: "Minimum Length".to_string(),
                    condition: "password.length >= 12".to_string(),
                    action: PolicyAction::Enforce,
                    parameters: HashMap::from([("min_length".to_string(), "12".to_string())]),
                },
                PolicyRule {
                    id: Uuid::new_v4(),
                    name: "Complexity Requirements".to_string(),
                    condition: "has_uppercase && has_lowercase && has_digit && has_special"
                        .to_string(),
                    action: PolicyAction::Enforce,
                    parameters: HashMap::new(),
                },
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            effective_date: Utc::now(),
            expiry_date: None,
            tags: vec!["security".to_string(), "authentication".to_string()],
        };

        self.policies.insert(password_policy.id, password_policy);

        let session_policy = SecurityPolicy {
            id: Uuid::new_v4(),
            name: "Session Timeout Policy".to_string(),
            description: "Enforces session timeout limits".to_string(),
            policy_type: PolicyType::SessionTimeout,
            status: PolicyStatus::Active,
            severity: PolicySeverity::Medium,
            rules: vec![PolicyRule {
                id: Uuid::new_v4(),
                name: "Maximum Session Duration".to_string(),
                condition: "session.duration <= 8_hours".to_string(),
                action: PolicyAction::Enforce,
                parameters: HashMap::from([("max_duration".to_string(), "28800".to_string())]),
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            effective_date: Utc::now(),
            expiry_date: None,
            tags: vec!["security".to_string(), "session".to_string()],
        };

        self.policies.insert(session_policy.id, session_policy);
    }

    pub fn add_policy(&mut self, policy: SecurityPolicy) -> Result<()> {
        if self.policies.contains_key(&policy.id) {
            return Err(anyhow!("Policy already exists"));
        }

        log::info!("Adding security policy: {}", policy.name);
        self.policies.insert(policy.id, policy);
        Ok(())
    }

    pub fn update_policy(&mut self, policy_id: Uuid, updates: SecurityPolicy) -> Result<()> {
        if let Some(existing) = self.policies.get_mut(&policy_id) {
            *existing = updates;
            existing.updated_at = Utc::now();
            log::info!("Updated policy: {}", existing.name);
            Ok(())
        } else {
            Err(anyhow!("Policy not found"))
        }
    }

    pub fn check_password_policy(&mut self, password: &str) -> PolicyCheckResult {
        let policy = self
            .policies
            .values()
            .find(|p| {
                p.policy_type == PolicyType::PasswordStrength && p.status == PolicyStatus::Active
            })
            .cloned();

        if let Some(policy) = policy {
            let mut violations = Vec::new();
            let mut warnings = Vec::new();

            if password.len() < 12 {
                violations.push(PolicyViolation {
                    id: Uuid::new_v4(),
                    policy_id: policy.id,
                    rule_id: policy.rules[0].id,
                    timestamp: Utc::now(),
                    user_id: None,
                    resource: "password".to_string(),
                    action_attempted: "set_password".to_string(),
                    violation_details: format!(
                        "Password length {} is less than required 12",
                        password.len()
                    ),
                    severity: PolicySeverity::High,
                    resolved: false,
                });
            }

            let has_uppercase = password.chars().any(|c| c.is_uppercase());
            let has_lowercase = password.chars().any(|c| c.is_lowercase());
            let has_digit = password.chars().any(|c| c.is_numeric());
            let has_special = password.chars().any(|c| !c.is_alphanumeric());

            if !(has_uppercase && has_lowercase && has_digit && has_special) {
                violations.push(PolicyViolation {
                    id: Uuid::new_v4(),
                    policy_id: policy.id,
                    rule_id: policy.rules[1].id,
                    timestamp: Utc::now(),
                    user_id: None,
                    resource: "password".to_string(),
                    action_attempted: "set_password".to_string(),
                    violation_details: "Password does not meet complexity requirements".to_string(),
                    severity: PolicySeverity::High,
                    resolved: false,
                });
            }

            if password.to_lowercase().contains("password") {
                warnings.push("Password contains the word 'password'".to_string());
            }

            let result = PolicyCheckResult {
                policy_id: policy.id,
                passed: violations.is_empty(),
                violations: violations.clone(),
                warnings,
                timestamp: Utc::now(),
            };

            self.violations.extend(violations);
            self.check_history.push(result.clone());

            result
        } else {
            PolicyCheckResult {
                policy_id: Uuid::nil(),
                passed: true,
                violations: Vec::new(),
                warnings: vec!["No password policy configured".to_string()],
                timestamp: Utc::now(),
            }
        }
    }

    pub fn check_session_policy(&mut self, session_duration_seconds: u64) -> PolicyCheckResult {
        let policy = self
            .policies
            .values()
            .find(|p| {
                p.policy_type == PolicyType::SessionTimeout && p.status == PolicyStatus::Active
            })
            .cloned();

        if let Some(policy) = policy {
            let mut violations = Vec::new();

            if session_duration_seconds > 28800 {
                violations.push(PolicyViolation {
                    id: Uuid::new_v4(),
                    policy_id: policy.id,
                    rule_id: policy.rules[0].id,
                    timestamp: Utc::now(),
                    user_id: None,
                    resource: "session".to_string(),
                    action_attempted: "extend_session".to_string(),
                    violation_details: format!(
                        "Session duration {} seconds exceeds maximum 28800 seconds",
                        session_duration_seconds
                    ),
                    severity: PolicySeverity::Medium,
                    resolved: false,
                });
            }

            let result = PolicyCheckResult {
                policy_id: policy.id,
                passed: violations.is_empty(),
                violations: violations.clone(),
                warnings: Vec::new(),
                timestamp: Utc::now(),
            };

            self.violations.extend(violations);
            self.check_history.push(result.clone());

            result
        } else {
            PolicyCheckResult {
                policy_id: Uuid::nil(),
                passed: true,
                violations: Vec::new(),
                warnings: vec!["No session policy configured".to_string()],
                timestamp: Utc::now(),
            }
        }
    }

    pub fn check_all_policies(&mut self, context: &PolicyContext) -> Vec<PolicyCheckResult> {
        let active_policy_ids: Vec<Uuid> = self
            .policies
            .iter()
            .filter(|(_, p)| p.status == PolicyStatus::Active)
            .map(|(id, _)| *id)
            .collect();

        let mut results = Vec::new();

        for policy_id in active_policy_ids {
            let result = self.check_policy(policy_id, context);
            if let Ok(result) = result {
                results.push(result);
            }
        }

        results
    }

    pub fn check_policy(
        &mut self,
        policy_id: Uuid,
        context: &PolicyContext,
    ) -> Result<PolicyCheckResult> {
        let policy = self
            .policies
            .get(&policy_id)
            .ok_or_else(|| anyhow!("Policy not found"))?
            .clone();

        let mut violations = Vec::new();
        let warnings = Vec::new();

        for rule in &policy.rules {
            if !Self::evaluate_rule(rule, context) {
                violations.push(PolicyViolation {
                    id: Uuid::new_v4(),
                    policy_id: policy.id,
                    rule_id: rule.id,
                    timestamp: Utc::now(),
                    user_id: context.user_id,
                    resource: context.resource.clone(),
                    action_attempted: context.action.clone(),
                    violation_details: format!("Rule '{}' failed", rule.name),
                    severity: policy.severity.clone(),
                    resolved: false,
                });
            }
        }

        let result = PolicyCheckResult {
            policy_id: policy.id,
            passed: violations.is_empty(),
            violations: violations.clone(),
            warnings,
            timestamp: Utc::now(),
        };

        self.violations.extend(violations);
        self.check_history.push(result.clone());

        Ok(result)
    }

    pub(super) fn evaluate_rule(rule: &PolicyRule, _context: &PolicyContext) -> bool {
        !matches!(rule.action, PolicyAction::Deny)
    }

    pub fn get_violations(&self, unresolved_only: bool) -> Vec<PolicyViolation> {
        if unresolved_only {
            self.violations
                .iter()
                .filter(|v| !v.resolved)
                .cloned()
                .collect()
        } else {
            self.violations.clone()
        }
    }

    pub fn resolve_violation(&mut self, violation_id: Uuid) -> Result<()> {
        if let Some(violation) = self.violations.iter_mut().find(|v| v.id == violation_id) {
            violation.resolved = true;
            log::info!("Resolved violation: {}", violation_id);
            Ok(())
        } else {
            Err(anyhow!("Violation not found"))
        }
    }

    pub fn get_compliance_report(&self) -> PolicyComplianceReport {
        let total_policies = self.policies.len();
        let active_policies = self
            .policies
            .values()
            .filter(|p| p.status == PolicyStatus::Active)
            .count();
        let total_violations = self.violations.len();
        let unresolved_violations = self.violations.iter().filter(|v| !v.resolved).count();
        let critical_violations = self
            .violations
            .iter()
            .filter(|v| v.severity == PolicySeverity::Critical)
            .count();

        let recent_checks = self
            .check_history
            .iter()
            .filter(|c| c.timestamp > Utc::now() - Duration::days(7))
            .count();

        let compliance_rate = if self.check_history.is_empty() {
            100.0
        } else {
            let passed = self.check_history.iter().filter(|c| c.passed).count();
            (passed as f64 / self.check_history.len() as f64) * 100.0
        };

        PolicyComplianceReport {
            generated_at: Utc::now(),
            total_policies,
            active_policies,
            total_violations,
            unresolved_violations,
            critical_violations,
            recent_checks,
            compliance_rate,
        }
    }
}
