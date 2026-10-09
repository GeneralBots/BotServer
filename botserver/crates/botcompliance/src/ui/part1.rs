use super::*;

pub async fn handle_compliance_dashboard_page(State(_pool): State<Arc<DbPool>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Compliance Dashboard</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
.container { max-width: 1400px; margin: 0 auto; padding: 24px; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
.header h1 { font-size: 28px; color: #1a1a1a; }
.btn { padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
.btn-primary { background: #0066cc; color: white; }
.btn-primary:hover { background: #0052a3; }
.stats-row { display: grid; grid-template-columns: repeat(5, 1fr); gap: 16px; margin-bottom: 24px; }
.stat-card { background: white; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
.stat-value { font-size: 32px; font-weight: 600; }
.stat-value.green { color: #2e7d32; }
.stat-value.yellow { color: #f9a825; }
.stat-value.red { color: #c62828; }
.stat-label { font-size: 13px; color: #666; margin-top: 4px; }
.stat-change { font-size: 12px; margin-top: 8px; }
.stat-change.positive { color: #2e7d32; }
.stat-change.negative { color: #c62828; }
.tabs { display: flex; gap: 4px; margin-bottom: 24px; border-bottom: 1px solid #e0e0e0; }
.tab { padding: 12px 24px; background: none; border: none; cursor: pointer; font-size: 14px; color: #666; border-bottom: 2px solid transparent; }
.tab.active { color: #0066cc; border-bottom-color: #0066cc; }
.content-grid { display: grid; grid-template-columns: 2fr 1fr; gap: 24px; }
.section { background: white; border-radius: 12px; padding: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); margin-bottom: 24px; }
.section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.section-title { font-size: 18px; font-weight: 600; }
.framework-card { display: flex; align-items: center; justify-content: space-between; padding: 16px; border: 1px solid #e0e0e0; border-radius: 8px; margin-bottom: 12px; cursor: pointer; }
.framework-card:hover { background: #f8f9fa; }
.framework-info { display: flex; align-items: center; gap: 16px; }
.framework-icon { width: 48px; height: 48px; border-radius: 8px; display: flex; align-items: center; justify-content: center; font-size: 20px; font-weight: 600; }
.framework-gdpr { background: #e3f2fd; color: #1565c0; }
.framework-soc2 { background: #f3e5f5; color: #7b1fa2; }
.framework-iso { background: #e8f5e9; color: #2e7d32; }
.framework-hipaa { background: #fff3e0; color: #ef6c00; }
.framework-pci { background: #fce4ec; color: #c2185b; }
.framework-name { font-weight: 600; font-size: 16px; }
.framework-meta { font-size: 13px; color: #666; }
.score-badge { padding: 8px 16px; border-radius: 20px; font-weight: 600; font-size: 14px; }
.score-high { background: #e8f5e9; color: #2e7d32; }
.score-medium { background: #fff3e0; color: #ef6c00; }
.score-low { background: #ffebee; color: #c62828; }
.issue-item { display: flex; align-items: flex-start; gap: 12px; padding: 12px 0; border-bottom: 1px solid #f0f0f0; }
.issue-item:last-child { border-bottom: none; }
.issue-severity { width: 8px; height: 8px; border-radius: 50%; margin-top: 6px; flex-shrink: 0; }
.severity-critical { background: #c62828; }
.severity-high { background: #ef6c00; }
.severity-medium { background: #f9a825; }
.severity-low { background: #66bb6a; }
.issue-content { flex: 1; }
.issue-title { font-weight: 500; font-size: 14px; margin-bottom: 4px; }
.issue-meta { font-size: 12px; color: #666; }
.empty-state { text-align: center; padding: 40px; color: #666; }
</style>
</head>
<body>
<div class="container">
<div class="header">
<h1>Compliance Dashboard</h1>
<button class="btn btn-primary" onclick="runAudit()">Run Compliance Check</button>
</div>
<div class="stats-row">
<div class="stat-card">
<div class="stat-value green" id="overallScore">--</div>
<div class="stat-label">Overall Score</div>
<div class="stat-change positive">+2.5% from last month</div>
</div>
<div class="stat-card">
<div class="stat-value" id="totalControls">0</div>
<div class="stat-label">Controls Checked</div>
</div>
<div class="stat-card">
<div class="stat-value green" id="compliantControls">0</div>
<div class="stat-label">Compliant</div>
</div>
<div class="stat-card">
<div class="stat-value yellow" id="partialControls">0</div>
<div class="stat-label">Partial</div>
</div>
<div class="stat-card">
<div class="stat-value red" id="openIssues">0</div>
<div class="stat-label">Open Issues</div>
</div>
</div>
<div class="tabs">
<button class="tab active" data-view="overview">Overview</button>
<button class="tab" data-view="frameworks">Frameworks</button>
<button class="tab" data-view="issues">Issues</button>
<button class="tab" data-view="audit-log">Audit Log</button>
<button class="tab" data-view="training">Training</button>
</div>
<div class="content-grid">
<div>
<div class="section">
<div class="section-header">
<h2 class="section-title">Compliance Frameworks</h2>
<button class="btn" style="padding: 6px 12px; font-size: 12px;" onclick="addFramework()">+ Add Framework</button>
</div>
<div id="frameworksList">
<div class="framework-card" onclick="openFramework('gdpr')">
<div class="framework-info">
<div class="framework-icon framework-gdpr">GDPR</div>
<div>
<div class="framework-name">GDPR</div>
<div class="framework-meta">General Data Protection Regulation • 12 controls</div>
</div>
</div>
<span class="score-badge score-high">95%</span>
</div>
<div class="framework-card" onclick="openFramework('soc2')">
<div class="framework-info">
<div class="framework-icon framework-soc2">SOC2</div>
<div>
<div class="framework-name">SOC 2 Type II</div>
<div class="framework-meta">Service Organization Control • 24 controls</div>
</div>
</div>
<span class="score-badge score-high">92%</span>
</div>
<div class="framework-card" onclick="openFramework('iso27001')">
<div class="framework-info">
<div class="framework-icon framework-iso">ISO</div>
<div>
<div class="framework-name">ISO 27001</div>
<div class="framework-meta">Information Security Management • 18 controls</div>
</div>
</div>
<span class="score-badge score-medium">78%</span>
</div>
</div>
</div>
<div class="section">
<div class="section-header">
<h2 class="section-title">Recent Audit Activity</h2>
<a href="/suite/compliance/audit-log" style="color: #0066cc; font-size: 14px; text-decoration: none;">View All →</a>
</div>
<div id="auditActivity">
<div class="empty-state">No recent audit activity</div>
</div>
</div>
</div>
<div>
<div class="section">
<div class="section-header">
<h2 class="section-title">Open Issues</h2>
<a href="/suite/compliance/issues" style="color: #0066cc; font-size: 14px; text-decoration: none;">View All →</a>
</div>
<div id="issuesList">
<div class="issue-item">
<div class="issue-severity severity-critical"></div>
<div class="issue-content">
<div class="issue-title">Data retention policy needs update</div>
<div class="issue-meta">GDPR • Due in 5 days</div>
</div>
</div>
<div class="issue-item">
<div class="issue-severity severity-high"></div>
<div class="issue-content">
<div class="issue-title">Access review overdue for 3 users</div>
<div class="issue-meta">SOC 2 • Due in 2 days</div>
</div>
</div>
<div class="issue-item">
<div class="issue-severity severity-medium"></div>
<div class="issue-content">
<div class="issue-title">Security training incomplete</div>
<div class="issue-meta">ISO 27001 • Due in 14 days</div>
</div>
</div>
</div>
</div>
<div class="section">
<div class="section-header">
<h2 class="section-title">Upcoming Reviews</h2>
</div>
<div id="upcomingReviews">
<div class="issue-item">
<div class="issue-content">
<div class="issue-title">Quarterly Access Review</div>
<div class="issue-meta">Jan 31, 2025</div>
</div>
</div>
<div class="issue-item">
<div class="issue-content">
<div class="issue-title">Annual Security Assessment</div>
<div class="issue-meta">Feb 15, 2025</div>
</div>
</div>
</div>
</div>
</div>
</div>
</div>
<div id="framework-modal" class="modal-overlay">
<div class="modal">
<div class="modal-header">
<h3>New Compliance Framework</h3>
<button class="modal-close" onclick="closeFrameworkModal()">&times;</button>
</div>
<div class="modal-body">
<div class="form-group">
<label>Name (LGPD, GDPR, SOC 2, ISO 27001, PCI-DSS, custom)</label>
<input type="text" id="fw-name" placeholder="e.g. LGPD">
</div>
<div class="form-group">
<label>Version</label>
<input type="text" id="fw-version" placeholder="1.0.0">
</div>
<div class="form-group">
<label>Description</label>
<textarea id="fw-description" rows="3" placeholder="Describe the framework scope..."></textarea>
</div>
</div>
<div class="modal-footer">
<button class="btn" onclick="closeFrameworkModal()">Cancel</button>
<button class="btn btn-primary" onclick="submitFramework()">Create Framework</button>
</div>
</div>
</div>
<style>
.modal-overlay { display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 1000; align-items: center; justify-content: center; }
.modal-overlay.visible { display: flex; }
.modal { background: white; border-radius: 12px; width: 480px; max-width: 90vw; box-shadow: 0 8px 30px rgba(0,0,0,0.2); }
.modal-header { display: flex; justify-content: space-between; align-items: center; padding: 16px 20px; border-bottom: 1px solid #e0e0e0; }
.modal-header h3 { font-size: 16px; }
.modal-close { background: none; border: none; font-size: 22px; cursor: pointer; color: #666; }
.modal-body { padding: 20px; }
.modal-footer { display: flex; justify-content: flex-end; gap: 8px; padding: 16px 20px; border-top: 1px solid #e0e0e0; }
.form-group { margin-bottom: 14px; }
.form-group label { display: block; font-size: 13px; color: #555; margin-bottom: 6px; }
.form-group input, .form-group textarea { width: 100%; padding: 10px 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; box-sizing: border-box; }
</style>
<script>
document.querySelectorAll('.tab').forEach(tab => {
tab.addEventListener('click', () => {
document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
tab.classList.add('active');
loadView(tab.dataset.view);
});
});

async function loadDashboard() {
try {
const response = await fetch('/api/compliance/report');
const report = await response.json();
if (report) {
document.getElementById('overallScore').textContent = Math.round(report.overall_score || 0) + '%';
document.getElementById('totalControls').textContent = report.total_controls_checked || 0;
document.getElementById('compliantControls').textContent = report.compliant_controls || 0;
document.getElementById('openIssues').textContent = report.total_issues || 0;
}
} catch (e) {
console.error('Failed to load dashboard:', e);
}
}

function closeFrameworkModal() {
document.getElementById('framework-modal').classList.remove('visible');
}

function loadView(view) {
switch(view) {
case 'issues':
window.location = '/suite/compliance/issues';
break;
case 'audit-log':
window.location = '/suite/compliance/audit-log';
break;
case 'training':
window.location = '/suite/compliance/training';
break;
}
}

function runAudit() {
if (confirm('Run a full compliance check? This may take a few minutes.')) {
fetch('/api/compliance/checks', {
method: 'POST',
headers: { 'Content-Type': 'application/json' },
body: JSON.stringify({ framework: 'gdpr' })
}).then(() => {
alert('Compliance check started');
loadDashboard();
});
}
}

function openFramework(name) {
window.location = `/suite/compliance/framework/${encodeURIComponent(name)}`;
}

function addFramework() {
document.getElementById('framework-modal').classList.add('visible');
}

async function submitFramework() {
const name = document.getElementById('fw-name').value.trim();
const version = document.getElementById('fw-version').value.trim() || '1.0.0';
const description = document.getElementById('fw-description').value.trim();
if (!name) { alert('Framework name is required'); return; }
try {
const response = await fetch('/api/compliance/frameworks', {
method: 'POST',
headers: { 'Content-Type': 'application/json' },
body: JSON.stringify({ name, version, description, framework_key: name.toLowerCase().replace(/\s+/g, '_') })
});
if (!response.ok) { const err = await response.json().catch(() => ({})); throw new Error(err.error || 'Failed to create framework'); }
const fw = await response.json();
document.getElementById('framework-modal').classList.remove('visible');
loadFrameworks();
openFramework(fw.name);
} catch (e) { alert(e.message); }
}

async function loadFrameworks() {
try {
const response = await fetch('/api/compliance/frameworks');
const frameworks = await response.json();
const list = document.getElementById('frameworksList');
if (!frameworks || frameworks.length === 0) {
list.innerHTML = '<div class="empty-state">No frameworks configured. Click \"+ Add Framework\" to create one.</div>';
return;
}
list.innerHTML = frameworks.map(fw => `
<div class="framework-card" onclick="openFramework('${fw.name}')">
<div class="framework-info">
<div class="framework-icon framework-gdpr">${(fw.name || '?').substring(0, 4).toUpperCase()}</div>
<div>
<div class="framework-name">${fw.name}</div>
<div class="framework-meta">v${fw.version || '1.0.0'} • ${fw.controls_count || 0} controls</div>
</div>
</div>
<span class="score-badge ${fw.status === 'archived' ? 'score-low' : 'score-medium'}">${fw.status || 'active'}</span>
</div>
`).join('');
} catch (e) { console.error('Failed to load frameworks:', e); }
}

loadFrameworks();
loadDashboard();
</script>
</body>
</html>"#;
    Html(html.to_string())
}
