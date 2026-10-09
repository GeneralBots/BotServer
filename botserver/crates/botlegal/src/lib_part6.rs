use super::*;

pub async fn handle_legal_detail_page(
    State(_pool): State<Arc<DbPool>>,
    Path(doc_id): Path<Uuid>,
) -> Html<String> {
    let doc_id_safe = doc_id;
    let html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Legal Document</title>
<style>
* {{ box-sizing: border-box; margin: 0; padding: 0; }}
body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }}
.container {{ max-width: 1000px; margin: 0 auto; padding: 24px; }}
.back-link {{ color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }}
.document-header {{ background: white; border-radius: 12px; padding: 32px; margin-bottom: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
.document-title {{ font-size: 28px; font-weight: 600; margin-bottom: 16px; }}
.document-meta {{ display: flex; gap: 24px; color: #666; margin-bottom: 16px; flex-wrap: wrap; }}
.document-status {{ display: inline-block; padding: 6px 16px; border-radius: 20px; font-size: 13px; font-weight: 500; }}
.status-active {{ background: #e8f5e9; color: #2e7d32; }}
.status-draft {{ background: #f5f5f5; color: #666; }}
.document-actions {{ display: flex; gap: 12px; margin-top: 20px; }}
.btn {{ padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }}
.btn-primary {{ background: #0066cc; color: white; }}
.btn-outline {{ background: white; border: 1px solid #ddd; color: #333; }}
.btn-danger {{ background: #ffebee; color: #c62828; }}
.document-content {{ background: white; border-radius: 12px; padding: 32px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
.content-body {{ line-height: 1.8; color: #333; }}
.content-body h2 {{ margin: 24px 0 12px; font-size: 20px; }}
.content-body h3 {{ margin: 20px 0 10px; font-size: 16px; }}
.content-body p {{ margin-bottom: 16px; }}
.content-body ul, .content-body ol {{ margin-bottom: 16px; padding-left: 24px; }}
.content-body li {{ margin-bottom: 8px; }}
.version-history {{ background: white; border-radius: 12px; padding: 24px; margin-top: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }}
.version-item {{ display: flex; justify-content: space-between; align-items: center; padding: 12px 0; border-bottom: 1px solid #f0f0f0; }}
.version-item:last-child {{ border-bottom: none; }}
.version-info {{ font-size: 14px; }}
.version-date {{ color: #666; font-size: 13px; }}
</style>
</head>
<body>
<div class="container">
<a href="/suite/legal" class="back-link">← Back to Legal Documents</a>
<div class="document-header">
<h1 class="document-title" id="docTitle">Loading...</h1>
<div class="document-meta">
<span id="docType">Document</span>
<span id="docVersion">Version 1.0</span>
<span id="docUpdated">Updated: -</span>
<span id="docExpiry"></span>
</div>
<span class="document-status status-draft" id="docStatus">Draft</span>
<div class="document-actions">
<button class="btn btn-primary" onclick="editDocument()">Edit Document</button>
<button class="btn btn-outline" onclick="downloadPdf()">Download PDF</button>
<button class="btn btn-outline" onclick="viewHistory()">Version History</button>
<button class="btn btn-danger" onclick="deleteDocument()">Delete</button>
</div>
</div>
<div class="document-content">
<div class="content-body" id="docContent">
<p>Loading document content...</p>
</div>
</div>
<div class="version-history" id="versionHistory" style="display: none;">
<h3 style="margin-bottom: 16px;">Version History</h3>
<div id="versionList"></div>
</div>
</div>
<script>
const docId = '{doc_id_safe}';

async function loadDocument() {{
try {{
const response = await fetch(`/api/legal/documents/${{docId}}`);
const doc = await response.json();
if (doc) {{
document.getElementById('docTitle').textContent = doc.title;
document.getElementById('docType').textContent = doc.document_type || 'Document';
document.getElementById('docVersion').textContent = `Version ${{doc.version || '1.0'}}`;
document.getElementById('docUpdated').textContent = `Updated: ${{new Date(doc.updated_at).toLocaleDateString()}}`;

if (doc.expiry_date) {{
document.getElementById('docExpiry').textContent = `Expires: ${{new Date(doc.expiry_date).toLocaleDateString()}}`;
}}

const statusEl = document.getElementById('docStatus');
statusEl.textContent = (doc.status || 'draft').charAt(0).toUpperCase() + (doc.status || 'draft').slice(1);
statusEl.className = `document-status status-${{doc.status || 'draft'}}`;

document.getElementById('docContent').innerHTML = doc.content || '<p>No content available</p>';
}}
}} catch (e) {{
console.error('Failed to load document:', e);
}}
}}

function editDocument() {{
window.location = `/suite/legal/${{docId}}/edit`;
}}

async function downloadPdf() {{
window.open(`/api/legal/documents/${{docId}}/pdf`, '_blank');
}}

function viewHistory() {{
const history = document.getElementById('versionHistory');
history.style.display = history.style.display === 'none' ? 'block' : 'none';
}}

async function deleteDocument() {{
if (!confirm('Are you sure you want to delete this document? This action cannot be undone.')) return;
try {{
await fetch(`/api/legal/documents/${{docId}}`, {{ method: 'DELETE' }});
window.location = '/suite/legal';
}} catch (e) {{
alert('Failed to delete document');
}}
}}

loadDocument();
</script>
</body>
</html>"#);
Html(html)
}
