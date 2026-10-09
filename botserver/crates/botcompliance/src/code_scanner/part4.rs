use super::*;

impl CodeScanner {
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let patterns = Self::build_patterns()?;
        Ok(Self {
            patterns,
            base_path: base_path.as_ref().to_path_buf(),
        })
    }

    pub(super) fn build_patterns() -> Result<Vec<ScanPattern>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![
            ScanPattern {
                regex: Regex::new(r#"(?i)password\s*=\s*["'][^"']+["']"#)?,
                issue_type: IssueType::PasswordInConfig,
                severity: IssueSeverity::Critical,
                title: "Hardcoded Password".to_string(),
                description: "A password is hardcoded in the source code. This is a critical security risk.".to_string(),
                remediation: "Move the password to Vault using: vault_password = GET VAULT SECRET \"password_key\"".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r#"(?i)(api[_-]?key|apikey|secret[_-]?key|client[_-]?secret)\s*=\s*["'][^"']{8,}["']"#)?,
                issue_type: IssueType::HardcodedSecret,
                severity: IssueSeverity::Critical,
                title: "Hardcoded API Key/Secret".to_string(),
                description: "An API key or secret is hardcoded in the source code.".to_string(),
                remediation: "Store secrets in Vault and retrieve with GET VAULT SECRET".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r#"(?i)token\s*=\s*["'][a-zA-Z0-9_\-]{20,}["']"#)?,
                issue_type: IssueType::HardcodedSecret,
                severity: IssueSeverity::High,
                title: "Hardcoded Token".to_string(),
                description: "A token appears to be hardcoded in the source code.".to_string(),
                remediation: "Store tokens securely in Vault".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"(?i)IF\s+.*\binput\b")?,
                issue_type: IssueType::DeprecatedIfInput,
                severity: IssueSeverity::Medium,
                title: "Deprecated IF...input Pattern".to_string(),
                description: "Using IF with raw input variable. Prefer HEAR AS for type-safe input handling.".to_string(),
                remediation: "Replace with: HEAR response AS STRING\nIF response = \"value\" THEN".to_string(),
                category: "Code Quality".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"(?i)\b(GET_BOT_MEMORY|SET_BOT_MEMORY|GET_USER_MEMORY|SET_USER_MEMORY|USE_KB|USE_TOOL|SEND_MAIL|CREATE_TASK)\b")?,
                issue_type: IssueType::UnderscoreInKeyword,
                severity: IssueSeverity::Low,
                title: "Underscore in Keyword".to_string(),
                description: "Keywords should use spaces instead of underscores for consistency.".to_string(),
                remediation: "Use spaces: GET BOT MEMORY, SET BOT MEMORY, etc.".to_string(),
                category: "Naming Convention".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"(?i)POST\s+TO\s+INSTAGRAM\s+\w+\s*,\s*\w+")?,
                issue_type: IssueType::InsecurePattern,
                severity: IssueSeverity::High,
                title: "Instagram Credentials in Code".to_string(),
                description: "Instagram username/password passed directly. Use secure credential storage.".to_string(),
                remediation: "Store Instagram credentials in Vault and retrieve securely.".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"(?i)(SELECT|INSERT|UPDATE|DELETE)\s+.*(FROM|INTO|SET)\s+")?,
                issue_type: IssueType::FragileCode,
                severity: IssueSeverity::Medium,
                title: "Raw SQL Query".to_string(),
                description: "Raw SQL queries in BASIC code may be vulnerable to injection.".to_string(),
                remediation: "Use parameterized queries or the built-in data operations (SAVE, GET, etc.)".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"(?i)\bEVAL\s*\(")?,
                issue_type: IssueType::FragileCode,
                severity: IssueSeverity::High,
                title: "Dynamic Code Execution".to_string(),
                description: "EVAL can execute arbitrary code and is a security risk.".to_string(),
                remediation: "Avoid EVAL. Use structured control flow instead.".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r#"(?i)(password|secret|key|token)\s*=\s*["'][A-Za-z0-9+/=]{40,}["']"#)?,
                issue_type: IssueType::HardcodedSecret,
                severity: IssueSeverity::High,
                title: "Potential Encoded Secret".to_string(),
                description: "A base64-like string is assigned to a sensitive variable.".to_string(),
                remediation: "Remove encoded secrets from code. Use Vault for secret management.".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"(?i)(AKIA[0-9A-Z]{16})")?,
                issue_type: IssueType::HardcodedSecret,
                severity: IssueSeverity::Critical,
                title: "AWS Access Key".to_string(),
                description: "An AWS access key ID is hardcoded in the source code.".to_string(),
                remediation: "Remove immediately and rotate the key. Use IAM roles or Vault.".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"-----BEGIN\s+(RSA\s+)?PRIVATE\s+KEY-----")?,
                issue_type: IssueType::HardcodedSecret,
                severity: IssueSeverity::Critical,
                title: "Private Key in Code".to_string(),
                description: "A private key is embedded in the source code.".to_string(),
                remediation: "Remove private key immediately. Store in secure key management system.".to_string(),
                category: "Security".to_string(),
            },
            ScanPattern {
                regex: Regex::new(r"(?i)(postgres|mysql|mongodb|redis)://[^:]+:[^@]+@")?,
                issue_type: IssueType::HardcodedSecret,
                severity: IssueSeverity::Critical,
                title: "Database Credentials in Connection String".to_string(),
                description: "Database connection string contains embedded credentials.".to_string(),
                remediation: "Use environment variables or Vault for database credentials.".to_string(),
                category: "Security".to_string(),
            },
        ])
    }

    pub async fn scan_all(&self) -> Result<ComplianceScanResult, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();
        let mut bot_results = Vec::new();
        let mut total_stats = ScanStats::default();
        let mut total_files = 0;

        let templates_path = self.base_path.join("templates");
        let work_path = self.base_path.join("work");

        let mut bot_paths = Vec::new();

        if templates_path.exists() {
            for entry in WalkDir::new(&templates_path).max_depth(3).into_iter().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name.ends_with(".gbai") || name.ends_with(".gbdialog") {
                        bot_paths.push(path.to_path_buf());
                    }
                }
            }
        }

        if work_path.exists() {
            for entry in WalkDir::new(&work_path).max_depth(3).into_iter().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name.ends_with(".gbai") || name.ends_with(".gbdialog") {
                        bot_paths.push(path.to_path_buf());
                    }
                }
            }
        }

        for bot_path in &bot_paths {
            let result = self.scan_bot(bot_path).await?;
            total_files += result.files_scanned;
            total_stats.merge(&result.stats);
            bot_results.push(result);
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(ComplianceScanResult {
            scanned_at: Utc::now(),
            duration_ms,
            bots_scanned: bot_results.len(),
            total_files,
            stats: total_stats,
            bot_results,
        })
    }

    pub async fn scan_bot(&self, bot_path: &Path) -> Result<BotScanResult, Box<dyn std::error::Error + Send + Sync>> {
        let bot_name = bot_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let bot_id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, bot_name.as_bytes()).to_string();

        let mut issues = Vec::new();
        let mut stats = ScanStats::default();
        let mut files_scanned = 0;

        for entry in WalkDir::new(bot_path).into_iter().flatten() {
            let path = entry.path();
            if path.is_file() {
                let extension = path.extension().unwrap_or_default().to_string_lossy();
                if extension == "bas" || extension == "csv" {
                    files_scanned += 1;
                    let file_issues = self.scan_file(path).await?;
                    for issue in file_issues {
                        stats.add_issue(&issue.severity);
                        issues.push(issue);
                    }
                }
            }
        }

        let config_path = bot_path.join("config.csv");
        if config_path.exists() {
            let vault_configured = self.check_vault_config(&config_path).await?;
            if !vault_configured {
                let issue = CodeIssue {
                    id: uuid::Uuid::new_v4().to_string(),
                    severity: IssueSeverity::Info,
                    issue_type: IssueType::MissingVault,
                    title: "Vault Not Configured".to_string(),
                    description: "This bot is not configured to use Vault for secrets management.".to_string(),
                    file_path: config_path.to_string_lossy().to_string(),
                    line_number: None,
                    code_snippet: None,
                    remediation: "Add VAULT_ADDR and VAULT_TOKEN to configuration for secure secret management.".to_string(),
                    category: "Configuration".to_string(),
                    detected_at: Utc::now(),
                };
                stats.add_issue(&issue.severity);
                issues.push(issue);
            }
        }

        issues.sort_by(|a, b| b.severity.cmp(&a.severity));

        Ok(BotScanResult {
            bot_id,
            bot_name,
            scanned_at: Utc::now(),
            files_scanned,
            issues,
            stats,
        })
    }

    async fn scan_file(&self, file_path: &Path) -> Result<Vec<CodeIssue>, Box<dyn std::error::Error + Send + Sync>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let mut issues = Vec::new();

        let relative_path = file_path.strip_prefix(&self.base_path).unwrap_or(file_path).to_string_lossy().to_string();

        for (line_number, line) in content.lines().enumerate() {
            let line_num = line_number + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("REM") || trimmed.starts_with('\'') || trimmed.starts_with("//") {
                continue;
            }

            for pattern in &self.patterns {
                if pattern.regex.is_match(line) {
                    let snippet = Self::redact_sensitive(line);
                    let issue = CodeIssue {
                        id: uuid::Uuid::new_v4().to_string(),
                        severity: pattern.severity.clone(),
                        issue_type: pattern.issue_type.clone(),
                        title: pattern.title.clone(),
                        description: pattern.description.clone(),
                        file_path: relative_path.clone(),
                        line_number: Some(line_num),
                        code_snippet: Some(snippet),
                        remediation: pattern.remediation.clone(),
                        category: pattern.category.clone(),
                        detected_at: Utc::now(),
                    };
                    issues.push(issue);
                }
            }
        }

        Ok(issues)
    }

    pub(super) fn redact_sensitive(line: &str) -> String {
        let mut result = line.to_string();

        if let Ok(secret_pattern) = Regex::new(r#"(["'])[^"']{8,}(["'])"#) {
            result = secret_pattern.replace_all(&result, "$1***REDACTED***$2").to_string();
        }

        if let Ok(aws_pattern) = Regex::new(r"AKIA[0-9A-Z]{16}") {
            result = aws_pattern.replace_all(&result, "AKIA***REDACTED***").to_string();
        }

        result
    }

    async fn check_vault_config(&self, config_path: &Path) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let content = tokio::fs::read_to_string(config_path).await?;
        let has_vault = content.to_lowercase().contains("vault_addr")
            || content.to_lowercase().contains("vault_token")
            || content.to_lowercase().contains("vault-");
        Ok(has_vault)
    }
}
