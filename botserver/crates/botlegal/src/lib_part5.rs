use super::*;

pub async fn handle_legal_list_page(State(_pool): State<Arc<DbPool>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Legal Documents</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
.container { max-width: 1400px; margin: 0 auto; padding: 24px; }
.header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
.header h1 { font-size: 28px; color: #1a1a1a; }
.btn { padding: 10px 20px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
.btn-primary { background: #0066cc; color: white; }
.btn-primary:hover { background: #0052a3; }
.tabs { display: flex; gap: 4px; margin-bottom: 24px; border-bottom: 1px solid #e0e0e0; }
.tab { padding: 12px 24px; background: none; border: none; cursor: pointer; font-size: 14px; color: #666; border-bottom: 2px solid transparent; }
.tab.active { color: #0066cc; border-bottom-color: #0066cc; }
.stats-row { display: grid; grid-template-columns: repeat(4, 1fr); gap: 16px; margin-bottom: 24px; }
.stat-card { background: white; border-radius: 12px; padding: 20px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
.stat-value { font-size: 28px; font-weight: 600; color: #1a1a1a; }
.stat-label { font-size: 13px; color: #666; margin-top: 4px; }
.document-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(350px, 1fr)); gap: 24px; }
.document-card { background: white; border-radius: 12px; padding: 24px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); cursor: pointer; transition: transform 0.2s, box-shadow 0.2s; }
.document-card:hover { transform: translateY(-2px); box-shadow: 0 4px 16px rgba(0,0,0,0.12); }
.document-icon { width: 48px; height: 48px; border-radius: 8px; background: #e3f2fd; display: flex; align-items: center; justify-content: center; font-size: 24px; margin-bottom: 16px; }
.document-title { font-size: 16px; font-weight: 600; color: #1a1a1a; margin-bottom: 8px; }
.document-meta { font-size: 13px; color: #666; margin-bottom: 12px; }
.document-status { display: inline-block; padding: 4px 12px; border-radius: 20px; font-size: 12px; font-weight: 500; }
.status-active { background: #e8f5e9; color: #2e7d32; }
.status-draft { background: #f5f5f5; color: #666; }
.status-expired { background: #ffebee; color: #c62828; }
.status-review { background: #fff3e0; color: #ef6c00; }
.filters { display: flex; gap: 12px; margin-bottom: 24px; }
.search-box { flex: 1; padding: 10px 16px; border: 1px solid #ddd; border-radius: 8px; }
.filter-select { padding: 8px 16px; border: 1px solid #ddd; border-radius: 8px; background: white; }
.empty-state { text-align: center; padding: 80px 24px; color: #666; }
.empty-state h3 { margin-bottom: 8px; color: #1a1a1a; }
</style>
</head>
<body>
<div class="container">
<div class="header">
<h1>Legal Documents</h1>
<button class="btn btn-primary" onclick="createDocument()">+ New Document</button>
</div>
<div class="stats-row">
<div class="stat-card">
<div class="stat-value" id="totalDocs">0</div>
<div class="stat-label">Total Documents</div>
</div>
<div class="stat-card">
<div class="stat-value" id="activeDocs">0</div>
<div class="stat-label">Active</div>
</div>
<div class="stat-card">
<div class="stat-value" id="pendingReview">0</div>
<div class="stat-label">Pending Review</div>
</div>
<div class="stat-card">
<div class="stat-value" id="expiringDocs">0</div>
<div class="stat-label">Expiring Soon</div>
</div>
</div>
<div class="tabs">
<button class="tab active" data-type="all">All Documents</button>
<button class="tab" data-type="policies">Policies</button>
<button class="tab" data-type="contracts">Contracts</button>
<button class="tab" data-type="agreements">Agreements</button>
<button class="tab" data-type="consents">Consent Forms</button>
</div>
<div class="filters">
<input type="text" class="search-box" placeholder="Search legal documents..." id="searchInput" oninput="filterDocuments()">
<select class="filter-select" id="statusFilter" onchange="filterDocuments()">
<option value="">All Status</option>
<option value="active">Active</option>
<option value="draft">Draft</option>
<option value="review">Under Review</option>
<option value="expired">Expired</option>
</select>
<select class="filter-select" id="sortBy" onchange="filterDocuments()">
<option value="updated">Recently Updated</option>
<option value="created">Recently Created</option>
<option value="name">Name A-Z</option>
<option value="expiry">Expiry Date</option>
</select>
</div>
<div class="document-grid" id="documentGrid">
<div class="empty-state">
<h3>No legal documents yet</h3>
<p>Create your first legal document to get started</p>
</div>
</div>
</div>
<script>
let documents = [];
let currentType = 'all';

document.querySelectorAll('.tab').forEach(tab => {
tab.addEventListener('click', () => {
document.querySelectorAll('.tab').forEach(t => t.classList.remove('active'));
tab.classList.add('active');
currentType = tab.dataset.type;
filterDocuments();
});
});

async function loadDocuments() {
try {
const response = await fetch('/api/legal/documents');
documents = await response.json();
renderDocuments(documents);
updateStats();
} catch (e) {
console.error('Failed to load documents:', e);
}
}

function renderDocuments(docs) {
const grid = document.getElementById('documentGrid');
if (!docs || docs.length === 0) {
grid.innerHTML = '<div class="empty-state"><h3>No documents found</h3><p>Create a new document to get started</p></div>';
return;
}

grid.innerHTML = docs.map(d => `
<div class="document-card" onclick="openDocument('${d.id}')">
<div class="document-icon">${getDocIcon(d.document_type)}</div>
<div class="document-title">${d.title}</div>
<div class="document-meta">
${d.document_type || 'Document'} • Version ${d.version || '1.0'} • Updated ${formatDate(d.updated_at)}
</div>
<span class="document-status status-${d.status || 'draft'}">${(d.status || 'draft').charAt(0).toUpperCase() + (d.status || 'draft').slice(1)}</span>
</div>
`).join('');
}

function getDocIcon(type) {
const icons = {
policy: '📋',
contract: '📝',
agreement: '🤝',
consent: '✅',
terms: '📜',
privacy: '🔒'
};
return icons[type] || '📄';
}

function formatDate(dateStr) {
if (!dateStr) return 'Never';
const date = new Date(dateStr);
const now = new Date();
const diff = now - date;
if (diff < 86400000) return 'Today';
if (diff < 172800000) return 'Yesterday';
return date.toLocaleDateString();
}

function updateStats() {
document.getElementById('totalDocs').textContent = documents.length;
document.getElementById('activeDocs').textContent = documents.filter(d => d.status === 'active').length;
document.getElementById('pendingReview').textContent = documents.filter(d => d.status === 'review').length;
const now = new Date();
const thirtyDays = 30 * 24 * 60 * 60 * 1000;
document.getElementById('expiringDocs').textContent = documents.filter(d => {
if (!d.expiry_date) return false;
const expiry = new Date(d.expiry_date);
return expiry - now < thirtyDays && expiry > now;
}).length;
}

function filterDocuments() {
const query = document.getElementById('searchInput').value.toLowerCase();
const status = document.getElementById('statusFilter').value;

let filtered = documents;

if (currentType !== 'all') {
filtered = filtered.filter(d => d.document_type === currentType.slice(0, -1));
}

if (query) {
filtered = filtered.filter(d =>
d.title.toLowerCase().includes(query) ||
(d.description && d.description.toLowerCase().includes(query))
);
}

if (status) {
filtered = filtered.filter(d => d.status === status);
}

renderDocuments(filtered);
}

function createDocument() {
window.location = '/suite/legal/new';
}

function openDocument(id) {
window.location = `/suite/legal/${id}`;
}

loadDocuments();
</script>
</body>
</html>"#;
Html(html.to_string())
}
