pub(crate) use chrono::{DateTime, Utc};
pub(crate) use regex::Regex;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    PasswordInConfig,
    HardcodedSecret,
    DeprecatedKeyword,
    FragileCode,
    ConfigurationIssue,
    UnderscoreInKeyword,
    MissingVault,
    InsecurePattern,
    DeprecatedIfInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeIssue {
    pub id: String,
    pub severity: IssueSeverity,
    pub issue_type: IssueType,
    pub title: String,
    pub description: String,
    pub file_path: String,
    pub line_number: Option<usize>,
    pub code_snippet: Option<String>,
    pub remediation: String,
    pub category: String,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotScanResult {
    pub bot_id: String,
    pub bot_name: String,
    pub scanned_at: DateTime<Utc>,
    pub files_scanned: usize,
    pub issues: Vec<CodeIssue>,
    pub stats: ScanStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStats {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceScanResult {
    pub scanned_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub bots_scanned: usize,
    pub total_files: usize,
    pub stats: ScanStats,
    pub bot_results: Vec<BotScanResult>,
}

struct ScanPattern {
    regex: Regex,
    issue_type: IssueType,
    severity: IssueSeverity,
    title: String,
    description: String,
    remediation: String,
    category: String,
}

pub struct CodeScanner {
    patterns: Vec<ScanPattern>,
    base_path: PathBuf,
}

pub struct ComplianceReporter;

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;
mod part6;
