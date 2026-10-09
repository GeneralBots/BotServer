use super::*;

/// `GET /suite/compliance/framework/:name`
///
/// Framework configuration page: controls catalog editor, evidence attachment
/// and coverage display. Driven by the `/api/compliance/frameworks/*` API.
pub async fn handle_compliance_framework_page(
    State(_pool): State<Arc<DbPool>>,
    Path(framework_name): Path<String>,
) -> Html<String> {
    let fw_name = framework_name.replace('+', " ");
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Framework Configuration</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
.container { max-width: 1200px; margin: 0 auto; padding: 24px; }
.back-link { color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-wrap: wrap; gap: 12px; }
.header h1 { font-size: 24px; }
.meta { color: #666; font-size: 14px; }
.btn { padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
.btn-primary { background: #0066cc; color: white; }
.btn-outline { background: white; border: 1px solid #ddd; color: #333; }
.btn-danger { background: #fff; border: 1px solid #fca5a5; color: #dc2626; }
.stats-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px; margin-bottom: 24px; }
.stat-card { background: white; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
.stat-value { font-size: 28px; font-weight: 600; }
.stat-value.green { color: #2e7d32; }
.stat-label { font-size: 13px; color: #666; margin-top: 4px; }
.section { background: white; border-radius: 12px; padding: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); margin-bottom: 24px; }
.section-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px; }
.section-title { font-size: 18px; font-weight: 600; }
.controls-table { width: 100%; border-collapse: collapse; }
.controls-table th, .controls-table td { text-align: left; padding: 12px; border-bottom: 1px solid #f0f0f0; font-size: 14px; }
.controls-table th { color: #666; font-size: 12px; text-transform: uppercase; letter-spacing: 0.5px; background: #fafafa; }
.badge { padding: 4px 10px; border-radius: 12px; font-size: 12px; font-weight: 500; }
.badge-active { background: #e8f5e9; color: #2e7d32; }
.badge-draft { background: #fff3e0; color: #ef6c00; }
.badge-approved { background: #e8f5e9; color: #2e7d32; }
.badge-mandatory { background: #f3e5f5; color: #7b1fa2; }
.badge-optional { background: #e3f2fd; color: #1565c0; }
.evidence-item { display: flex; align-items: center; justify-content: space-between; padding: 8px 12px; border: 1px solid #eee; border-radius: 8px; margin-top: 6px; font-size: 13px; }
.evidence-meta { color: #666; font-size: 12px; }
.empty-state { text-align: center; padding: 40px; color: #666; }
.modal-overlay { display: none; position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 1000; align-items: center; justify-content: center; }
.modal-overlay.visible { display: flex; }
.modal { background: white; border-radius: 12px; width: 520px; max-width: 92vw; box-shadow: 0 8px 30px rgba(0,0,0,0.2); }
.modal-header { display: flex; justify-content: space-between; align-items: center; padding: 16px 20px; border-bottom: 1px solid #e0e0e0; }
.modal-close { background: none; border: none; font-size: 22px; cursor: pointer; color: #666; }
.modal-body { padding: 20px; }
.modal-footer { display: flex; justify-content: flex-end; gap: 8px; padding: 16px 20px; border-top: 1px solid #e0e0e0; }
.form-group { margin-bottom: 14px; }
.form-group label { display: block; font-size: 13px; color: #555; margin-bottom: 6px; }
.form-group input, .form-group textarea, .form-group select { width: 100%; padding: 10px 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; box-sizing: border-box; }
</style>
</head>
<body>
<div class="container">
<a href="/suite/compliance" class="back-link">← Back to Compliance</a>
<div class="header">
<div>
<h1 id="fwTitle">Loading framework...</h1>
<div class="meta" id="fwMeta"></div>
</div>
<div style="display:flex;gap:8px">
<button class="btn btn-outline" onclick="exportCsv()">Export CSV</button>
<button class="btn" onclick="archiveFramework()">Archive</button>
</div>
</div>
<div class="stats-row">
<div class="stat-card"><div class="stat-value" id="statTotal">0</div><div class="stat-label">Total Controls</div></div>
<div class="stat-card"><div class="stat-value green" id="statCovered">0</div><div class="stat-label">Controls With Evidence</div></div>
<div class="stat-card"><div class="stat-value green" id="statCoverage">0%</div><div class="stat-label">Coverage</div></div>
</div>
<div class="section">
<div class="section-header">
<h2 class="section-title">Controls</h2>
<button class="btn btn-primary" style="padding:8px 14px;font-size:13px" onclick="showControlModal()">+ Add Control</button>
</div>
<div id="controlsList">
<div class="empty-state">Loading controls...</div>
</div>
</div>
</div>

<div class="modal-overlay" id="control-modal">
<div class="modal">
<div class="modal-header"><h3>Add Control</h3><button class="modal-close" onclick="hideModal('control-modal')">&times;</button></div>
<div class="modal-body">
<div class="form-group"><label>Control ID</label><input type="text" id="ctl-id" placeholder="e.g. CC6.1"></div>
<div class="form-group"><label>Title</label><input type="text" id="ctl-title" placeholder="Control title"></div>
<div class="form-group"><label>Category</label><input type="text" id="ctl-category" placeholder="e.g. Security"></div>
<div class="form-group"><label>Description</label><textarea id="ctl-desc" rows="2"></textarea></div>
<div class="form-group"><label>Mandatory</label><select id="ctl-mandatory"><option value="true">Mandatory</option><option value="false">Optional</option></select></div>
</div>
<div class="modal-footer"><button class="btn" onclick="hideModal('control-modal')">Cancel</button><button class="btn btn-primary" onclick="createControl()">Add Control</button></div>
</div>
</div>

<div class="modal-overlay" id="evidence-modal">
<div class="modal">
<div class="modal-header"><h3>Attach Evidence</h3><button class="modal-close" onclick="hideModal('evidence-modal')">&times;</button></div>
<div class="modal-body">
<div class="form-group"><label>File path (drive artifact)</label><input type="text" id="ev-path" placeholder="docs/policies/access-control.pdf"></div>
<div class="form-group"><label>Description</label><input type="text" id="ev-desc" placeholder="What this artifact proves"></div>
<div class="form-group"><label>Type</label><select id="ev-type"><option value="artifact">Artifact</option><option value="policy">Policy</option><option value="procedure">Procedure</option><option value="report">Report</option><option value="screenshot">Screenshot</option><option value="log">Log</option><option value="certificate">Certificate</option></select></div>
</div>
<div class="modal-footer"><button class="btn" onclick="hideModal('evidence-modal')">Cancel</button><button class="btn btn-primary" onclick="attachEvidence()">Attach</button></div>
</div>
</div>

<script>
const frameworkName = '__FW_NAME__';
let framework = null;

async function loadFramework() {
try {
const res = await fetch('/api/compliance/frameworks');
if (!res.ok) throw new Error('HTTP ' + res.status);
const frameworks = await res.json();
framework = (frameworks || []).find(f => f.name === frameworkName || f.framework_key === frameworkName.toLowerCase().replace(/\s+/g, '_'));
if (!framework) throw new Error('Framework not found');
document.getElementById('fwTitle').textContent = framework.name;
document.getElementById('fwMeta').textContent = 'v' + (framework.version || '1.0.0') + ' • ' + (framework.framework_key || '') + ' • ' + (framework.status || 'active');
await loadFrameworkDetail(framework.id);
} catch (e) {
document.getElementById('fwTitle').textContent = 'Framework not found';
document.getElementById('controlsList').innerHTML = '<div class="empty-state">' + e.message + '</div>';
}
}

async function loadFrameworkDetail(id) {
const res = await fetch('/api/compliance/frameworks/' + id);
if (!res.ok) throw new Error('HTTP ' + res.status);
framework = await res.json();
document.getElementById('statTotal').textContent = framework.total_controls || 0;
document.getElementById('statCovered').textContent = framework.controls_with_evidence || 0;
document.getElementById('statCoverage').textContent = (framework.coverage_percent || 0) + '%';
renderControls(framework.controls || []);
}

function renderControls(controls) {
const list = document.getElementById('controlsList');
if (!controls.length) { list.innerHTML = '<div class="empty-state">No controls yet. Add the first control for this framework.</div>'; return; }
list.innerHTML = '<table class="controls-table"><thead><tr><th>ID</th><th>Title</th><th>Category</th><th>Type</th><th>Evidence</th><th>Actions</th></tr></thead><tbody>'
+ controls.map(c => `
<tr>
<td><strong>${c.control_id}</strong></td>
<td>${c.title}${c.description ? '<br><small class="meta">' + c.description + '</small>' : ''}</td>
<td>${c.category || '-'}</td>
<td><span class="badge ${c.is_mandatory ? 'badge-mandatory' : 'badge-optional'}">${c.is_mandatory ? 'Mandatory' : 'Optional'}</span></td>
<td>
<span class="badge ${c.has_evidence ? 'badge-approved' : 'badge-draft'}">${c.has_evidence ? 'Covered' : 'No evidence'}</span>
${(c.evidence || []).map(e => `<div class="evidence-item"><span>📎 ${e.file_path}</span><span class="evidence-meta">${e.status}${e.approved_by ? ' • approved' : ''} <button class="btn btn-outline" style="padding:2px 8px;font-size:11px" onclick="approveEvidence('${e.id}')">Approve</button></span></div>`).join('')}
</td>
<td><button class="btn btn-outline" style="padding:4px 10px;font-size:12px" onclick="showEvidenceModal('${c.id}')">+ Evidence</button></td>
</tr>`).join('') + '</tbody></table>';
}

function showControlModal() { document.getElementById('control-modal').classList.add('visible'); }
function hideModal(id) { document.getElementById(id).classList.remove('visible'); }
let evidenceControlId = null;
function showEvidenceModal(controlId) { evidenceControlId = controlId; document.getElementById('evidence-modal').classList.add('visible'); }

async function createControl() {
const control_id = document.getElementById('ctl-id').value.trim();
const title = document.getElementById('ctl-title').value.trim();
if (!control_id || !title) { alert('Control ID and title are required'); return; }
const body = {
framework_id: framework.id,
control_id,
title,
category: document.getElementById('ctl-category').value.trim() || null,
description: document.getElementById('ctl-desc').value.trim() || null,
is_mandatory: document.getElementById('ctl-mandatory').value === 'true'
};
const res = await fetch('/api/compliance/controls', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
if (!res.ok) { const err = await res.json().catch(() => ({})); alert(err.error || 'Failed to create control'); return; }
hideModal('control-modal');
document.getElementById('ctl-id').value = ''; document.getElementById('ctl-title').value = ''; document.getElementById('ctl-desc').value = '';
await loadFrameworkDetail(framework.id);
}

async function attachEvidence() {
const file_path = document.getElementById('ev-path').value.trim();
if (!file_path || !evidenceControlId) { alert('File path is required'); return; }
const body = {
control_id: evidenceControlId,
file_path,
description: document.getElementById('ev-desc').value.trim() || null,
evidence_type: document.getElementById('ev-type').value
};
const res = await fetch('/api/compliance/evidence/attach', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) });
if (!res.ok) { const err = await res.json().catch(() => ({})); alert(err.error || 'Failed to attach evidence'); return; }
hideModal('evidence-modal');
document.getElementById('ev-path').value = ''; document.getElementById('ev-desc').value = '';
await loadFrameworkDetail(framework.id);
}

async function approveEvidence(evidenceId) {
const res = await fetch('/api/compliance/evidence/' + evidenceId + '/approve', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({}) });
if (!res.ok) { alert('Failed to approve evidence'); return; }
await loadFrameworkDetail(framework.id);
}

async function archiveFramework() {
if (!confirm('Archive this framework? Controls are preserved but it stops appearing as active.')) return;
const res = await fetch('/api/compliance/frameworks/' + framework.id + '/archive', { method: 'POST' });
if (!res.ok) { alert('Failed to archive framework'); return; }
window.location = '/suite/compliance';
}

function exportCsv() {
window.location = '/api/compliance/frameworks/' + framework.id + '/export.csv';
}

loadFramework();
</script>
</body>
</html>"#;
    let html = html.replace("__FW_NAME__", &fw_name);
    Html(html)
}
pub fn configure_compliance_ui_routes() -> Router<Arc<DbPool>> {
    Router::new()
        .route("/suite/compliance", get(handle_compliance_dashboard_page))
        .route("/suite/compliance/issues", get(handle_compliance_issues_page))
        .route("/suite/compliance/issues/:id", get(handle_compliance_issue_detail_page))
        .route("/suite/compliance/framework/:name", get(handle_compliance_framework_page))
}
