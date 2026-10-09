use super::*;

pub async fn handle_compliance_issues_page(State(_pool): State<Arc<DbPool>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Compliance Issues</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
.container { max-width: 1200px; margin: 0 auto; padding: 24px; }
.back-link { color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
.header h1 { font-size: 24px; }
.btn { padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
.btn-primary { background: #0066cc; color: white; }
.filters { display: flex; gap: 12px; margin-bottom: 24px; }
.search-box { flex: 1; padding: 10px 16px; border: 1px solid #ddd; border-radius: 8px; }
.filter-select { padding: 8px 16px; border: 1px solid #ddd; border-radius: 8px; background: white; }
.issues-table { background: white; border-radius: 12px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); overflow: hidden; }
.table-header { display: grid; grid-template-columns: 40px 1fr 120px 120px 120px 100px; padding: 16px 20px; background: #f9f9f9; font-weight: 600; font-size: 13px; color: #666; border-bottom: 1px solid #e0e0e0; }
.table-row { display: grid; grid-template-columns: 40px 1fr 120px 120px 120px 100px; padding: 16px 20px; border-bottom: 1px solid #f0f0f0; align-items: center; cursor: pointer; }
.table-row:hover { background: #f8f9fa; }
.severity-dot { width: 10px; height: 10px; border-radius: 50%; }
.severity-critical { background: #c62828; }
.severity-high { background: #ef6c00; }
.severity-medium { background: #f9a825; }
.severity-low { background: #66bb6a; }
.issue-title { font-weight: 500; }
.issue-framework { font-size: 12px; color: #666; margin-top: 4px; }
.status-badge { padding: 4px 10px; border-radius: 12px; font-size: 12px; font-weight: 500; }
.status-open { background: #ffebee; color: #c62828; }
.status-in-progress { background: #fff3e0; color: #ef6c00; }
.status-resolved { background: #e8f5e9; color: #2e7d32; }
.empty-state { text-align: center; padding: 60px; color: #666; }
</style>
</head>
<body>
<div class="container">
<a href="/suite/compliance" class="back-link">← Back to Compliance</a>
<div class="header">
<h1>Compliance Issues</h1>
<button class="btn btn-primary" onclick="createIssue()">+ Report Issue</button>
</div>
<div class="filters">
<input type="text" class="search-box" placeholder="Search issues..." id="searchInput">
<select class="filter-select" id="severityFilter">
<option value="">All Severities</option>
<option value="critical">Critical</option>
<option value="high">High</option>
<option value="medium">Medium</option>
<option value="low">Low</option>
</select>
<select class="filter-select" id="statusFilter">
<option value="">All Status</option>
<option value="open">Open</option>
<option value="in_progress">In Progress</option>
<option value="resolved">Resolved</option>
</select>
<select class="filter-select" id="frameworkFilter">
<option value="">All Frameworks</option>
<option value="gdpr">GDPR</option>
<option value="soc2">SOC 2</option>
<option value="iso27001">ISO 27001</option>
<option value="hipaa">HIPAA</option>
</select>
</div>
<div class="issues-table">
<div class="table-header">
<span></span>
<span>Issue</span>
<span>Framework</span>
<span>Status</span>
<span>Due Date</span>
<span>Assignee</span>
</div>
<div id="issuesList">
<div class="empty-state">Loading issues...</div>
</div>
</div>
</div>
<script>
async function loadIssues() {
try {
const response = await fetch('/api/compliance/issues');
const issues = await response.json();
renderIssues(issues);
} catch (e) {
console.error('Failed to load issues:', e);
document.getElementById('issuesList').innerHTML = '<div class="empty-state">Failed to load issues</div>';
}
}

function renderIssues(issues) {
const list = document.getElementById('issuesList');
if (!issues || issues.length === 0) {
list.innerHTML = '<div class="empty-state">No compliance issues found</div>';
return;
}

list.innerHTML = issues.map(i => `
<div class="table-row" onclick="openIssue('${i.id}')">
<div class="severity-dot severity-${i.severity || 'medium'}"></div>
<div>
<div class="issue-title">${i.title}</div>
<div class="issue-framework">${i.description ? i.description.substring(0, 60) + '...' : ''}</div>
</div>
<span>${i.framework || '-'}</span>
<span class="status-badge status-${(i.status || 'open').replace(' ', '-')}">${i.status || 'Open'}</span>
<span>${i.due_date ? new Date(i.due_date).toLocaleDateString() : '-'}</span>
<span>${i.assigned_to || 'Unassigned'}</span>
</div>
`).join('');
}

function openIssue(id) {
window.location = `/suite/compliance/issues/${id}`;
}

function createIssue() {
window.location = '/suite/compliance/issues/new';
}

loadIssues();
</script>
</body>
</html>"#;
    Html(html.to_string())
}
pub async fn handle_compliance_issue_detail_page(
    State(_pool): State<Arc<DbPool>>,
    Path(issue_id): Path<Uuid>,
) -> Html<String> {
    let issue_id_safe = issue_id;
    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Compliance Issue</title>
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }}
.container {{ max-width: 900px; margin: 0 auto; padding: 24px; }}
.back-link {{ color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }}
.issue-card {{ background: white; border-radius: 12px; padding: 32px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); margin-bottom: 24px; }}
.issue-header {{ display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 20px; }}
.issue-title {{ font-size: 24px; font-weight: 600; margin-bottom: 12px; }}
.issue-meta {{ display: flex; gap: 16px; flex-wrap: wrap; }}
.meta-item {{ font-size: 13px; color: #666; }}
.severity-badge {{ padding: 4px 12px; border-radius: 12px; font-size: 12px; font-weight: 500; }}
.severity-critical {{ background: #ffebee; color: #c62828; }}
.severity-high {{ background: #fff3e0; color: #ef6c00; }}
.severity-medium {{ background: #fff8e1; color: #f9a825; }}
.severity-low {{ background: #e8f5e9; color: #2e7d32; }}
.issue-description {{ line-height: 1.7; color: #444; margin-bottom: 20px; }}
.section {{ margin-top: 24px; padding-top: 24px; border-top: 1px solid #e0e0e0; }}
.section-title {{ font-size: 16px; font-weight: 600; margin-bottom: 12px; }}
.btn {{ padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }}
.btn-primary {{ background: #0066cc; color: white; }}
.btn-success {{ background: #2e7d32; color: white; }}
.btn-outline {{ background: white; border: 1px solid #ddd; color: #333; }}
.actions {{ display: flex; gap: 12px; }}
.remediation-box {{ background: #f9f9f9; border-radius: 8px; padding: 16px; line-height: 1.6; }}
</style>
</head>
<body>
<div class="container">
<a href="/suite/compliance/issues" class="back-link">← Back to Issues</a>
<div class="issue-card">
<div class="issue-header">
<div>
<h1 class="issue-title" id="issueTitle">Loading...</h1>
<div class="issue-meta">
<span class="severity-badge severity-medium" id="issueSeverity">Medium</span>
<span class="meta-item" id="issueFramework">Framework: -</span>
<span class="meta-item" id="issueStatus">Status: Open</span>
<span class="meta-item" id="issueDue">Due: -</span>
</div>
</div>
<div class="actions">
<button class="btn btn-success" onclick="resolveIssue()">Mark Resolved</button>
<button class="btn btn-outline" onclick="editIssue()">Edit</button>
</div>
</div>
<div class="issue-description" id="issueDescription">
Loading issue details...
</div>
<div class="section">
<h3 class="section-title">Remediation Steps</h3>
<div class="remediation-box" id="issueRemediation">
No remediation steps provided.
</div>
</div>
<div class="section">
<h3 class="section-title">Assignment</h3>
<p id="issueAssignee">Unassigned</p>
</div>
</div>
</div>
<script>
const issueId = '{issue_id_safe}';

async function loadIssue() {{
try {{
const response = await fetch(`/api/compliance/issues`);
const issues = await response.json();
const issue = issues.find(i => i.id === issueId);
if (issue) {{
document.getElementById('issueTitle').textContent = issue.title;
document.getElementById('issueDescription').textContent = issue.description || 'No description provided.';
document.getElementById('issueRemediation').textContent = issue.remediation || 'No remediation steps provided.';

const severityEl = document.getElementById('issueSeverity');
severityEl.textContent = (issue.severity || 'medium').charAt(0).toUpperCase() + (issue.severity || 'medium').slice(1);
severityEl.className = `severity-badge severity-${{issue.severity || 'medium'}}`;

document.getElementById('issueFramework').textContent = `Framework: ${{issue.framework || '-'}}`;
document.getElementById('issueStatus').textContent = `Status: ${{issue.status || 'Open'}}`;
document.getElementById('issueDue').textContent = issue.due_date ? `Due: ${{new Date(issue.due_date).toLocaleDateString()}}` : 'Due: -';
document.getElementById('issueAssignee').textContent = issue.assigned_to || 'Unassigned';
}}
}} catch (e) {{
console.error('Failed to load issue:', e);
}}
}}

async function resolveIssue() {{
if (!confirm('Mark this issue as resolved?')) return;
try {{
await fetch(`/api/compliance/issues/${{issueId}}`, {{
method: 'PUT',
headers: {{ 'Content-Type': 'application/json' }},
body: JSON.stringify({{ status: 'resolved' }})
}});
window.location = '/suite/compliance/issues';
}} catch (e) {{
alert('Failed to update issue');
}}
}}

function editIssue() {{
window.location = `/suite/compliance/issues/${{issueId}}/edit`;
}}

loadIssue();
</script>
</body>
</html>"#);
    Html(html)
}
