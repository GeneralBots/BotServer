use super::*;

impl ComplianceReporter {
    pub fn to_html(result: &ComplianceScanResult) -> String {
        let mut html = String::new();

        html.push_str("<!DOCTYPE html><html><head><title>Compliance Report</title>");
        html.push_str("<style>body{font-family:system-ui;margin:20px;}table{border-collapse:collapse;width:100%;}th,td{border:1px solid #ddd;padding:8px;text-align:left;}.critical{color:#dc2626;}.high{color:#ea580c;}.medium{color:#d97706;}.low{color:#65a30d;}.info{color:#0891b2;}</style>");
        html.push_str("</head><body>");

        html.push_str("<h1>Compliance Scan Report</h1>");
        use std::fmt::Write;
        let _ = write!(html, "<p>Scanned at: {}</p>", result.scanned_at);
        let _ = write!(html, "<p>Duration: {}ms</p>", result.duration_ms);
        let _ = write!(html, "<p>Bots scanned: {}</p>", result.bots_scanned);
        let _ = write!(html, "<p>Files scanned: {}</p>", result.total_files);

        html.push_str("<h2>Summary</h2>");
        let _ = write!(html, "<p class='critical'>Critical: {}</p>", result.stats.critical);
        let _ = write!(html, "<p class='high'>High: {}</p>", result.stats.high);
        let _ = write!(html, "<p class='medium'>Medium: {}</p>", result.stats.medium);
        let _ = write!(html, "<p class='low'>Low: {}</p>", result.stats.low);
        let _ = write!(html, "<p class='info'>Info: {}</p>", result.stats.info);

        html.push_str("<h2>Issues</h2>");
        html.push_str("<table><tr><th>Severity</th><th>Type</th><th>File</th><th>Line</th><th>Description</th></tr>");

        for bot in &result.bot_results {
            for issue in &bot.issues {
                let _ = write!(
                    html,
                    "<tr><td class='{}'>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    issue.severity,
                    issue.severity,
                    issue.issue_type,
                    issue.file_path,
                    issue.line_number.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string()),
                    issue.description
                );
            }
        }

        html.push_str("</table></body></html>");
        html
    }

    pub fn to_json(result: &ComplianceScanResult) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(result)
    }

    pub fn to_csv(result: &ComplianceScanResult) -> String {
        let mut csv = String::new();
        csv.push_str("Severity,Type,Category,File,Line,Title,Description,Remediation\n");

        for bot in &result.bot_results {
            for issue in &bot.issues {
                use std::fmt::Write;
                let _ = writeln!(
                    csv,
                    "{},{},{},{},{},{},{},{}",
                    issue.severity,
                    issue.issue_type,
                    issue.category,
                    issue.file_path,
                    issue.line_number.map(|n| n.to_string()).unwrap_or_else(|| "-".to_string()),
                    escape_csv(&issue.title),
                    escape_csv(&issue.description),
                    escape_csv(&issue.remediation)
                );
            }
        }

        csv
    }
}
pub(super) fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
