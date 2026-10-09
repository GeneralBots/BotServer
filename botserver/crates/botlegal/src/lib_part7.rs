use super::*;

pub async fn handle_legal_new_page(State(_pool): State<Arc<DbPool>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Create Legal Document</title>
<style>
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; }
.container { max-width: 900px; margin: 0 auto; padding: 24px; }
.back-link { color: #0066cc; text-decoration: none; display: inline-block; margin-bottom: 16px; }
.form-card { background: white; border-radius: 12px; padding: 32px; box-shadow: 0 2px 8px rgba(0,0,0,0.08); }
h1 { font-size: 24px; margin-bottom: 24px; }
.form-group { margin-bottom: 20px; }
.form-group label { display: block; font-weight: 500; margin-bottom: 8px; }
.form-group input, .form-group textarea, .form-group select { width: 100%; padding: 12px; border: 1px solid #ddd; border-radius: 8px; font-size: 14px; }
.form-group textarea { min-height: 300px; resize: vertical; font-family: inherit; }
.form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
.btn { padding: 12px 24px; border: none; border-radius: 8px; cursor: pointer; font-size: 14px; font-weight: 500; }
.btn-primary { background: #0066cc; color: white; }
.btn-primary:hover { background: #0052a3; }
.btn-secondary { background: #f5f5f5; color: #333; }
.form-actions { display: flex; gap: 12px; justify-content: flex-end; }
.template-buttons { display: flex; gap: 8px; flex-wrap: wrap; margin-bottom: 16px; }
.template-btn { padding: 8px 16px; border: 1px solid #ddd; border-radius: 6px; background: white; cursor: pointer; font-size: 13px; }
.template-btn:hover { background: #f5f5f5; border-color: #0066cc; }
</style>
</head>
<body>
<div class="container">
<a href="/suite/legal" class="back-link">← Back to Legal Documents</a>
<div class="form-card">
<h1>Create Legal Document</h1>
<form id="documentForm">
<div class="form-row">
<div class="form-group">
<label>Document Type</label>
<select id="documentType" required>
<option value="">Select type...</option>
<option value="policy">Policy</option>
<option value="contract">Contract</option>
<option value="agreement">Agreement</option>
<option value="consent">Consent Form</option>
<option value="terms">Terms of Service</option>
<option value="privacy">Privacy Policy</option>
</select>
</div>
<div class="form-group">
<label>Status</label>
<select id="status">
<option value="draft">Draft</option>
<option value="review">Under Review</option>
<option value="active">Active</option>
</select>
</div>
</div>
<div class="form-group">
<label>Document Title</label>
<input type="text" id="title" required placeholder="Enter document title">
</div>
<div class="form-group">
<label>Description</label>
<input type="text" id="description" placeholder="Brief description of this document">
</div>
<div class="form-row">
<div class="form-group">
<label>Effective Date</label>
<input type="date" id="effectiveDate">
</div>
<div class="form-group">
<label>Expiry Date (optional)</label>
<input type="date" id="expiryDate">
</div>
</div>
<div class="form-group">
<label>Document Content</label>
<div class="template-buttons">
<button type="button" class="template-btn" onclick="insertTemplate('privacy')">Privacy Policy Template</button>
<button type="button" class="template-btn" onclick="insertTemplate('terms')">Terms of Service Template</button>
<button type="button" class="template-btn" onclick="insertTemplate('cookie')">Cookie Policy Template</button>
</div>
<textarea id="content" placeholder="Enter the document content here..."></textarea>
</div>
<div class="form-actions">
<button type="button" class="btn btn-secondary" onclick="saveDraft()">Save as Draft</button>
<button type="button" class="btn btn-secondary" onclick="window.location='/suite/legal'">Cancel</button>
<button type="submit" class="btn btn-primary">Create Document</button>
</div>
</form>
</div>
</div>
<script>
const templates = {
privacy: `<h2>Privacy Policy</h2>
<p>Last updated: [DATE]</p>

<h3>1. Information We Collect</h3>
<p>We collect information you provide directly to us, including...</p>

<h3>2. How We Use Your Information</h3>
<p>We use the information we collect to...</p>

<h3>3. Information Sharing</h3>
<p>We do not share your personal information except...</p>

<h3>4. Data Security</h3>
<p>We implement appropriate security measures to protect...</p>

<h3>5. Your Rights</h3>
<p>You have the right to access, correct, or delete your personal data...</p>

<h3>6. Contact Us</h3>
<p>If you have questions about this Privacy Policy, please contact us at...</p>`,
terms: `<h2>Terms of Service</h2>
<p>Last updated: [DATE]</p>

<h3>1. Acceptance of Terms</h3>
<p>By accessing or using our services, you agree to be bound by these Terms...</p>

<h3>2. Use of Services</h3>
<p>You may use our services only in compliance with these Terms...</p>

<h3>3. User Accounts</h3>
<p>You are responsible for maintaining the confidentiality of your account...</p>

<h3>4. Intellectual Property</h3>
<p>All content and materials available through our services are protected...</p>

<h3>5. Limitation of Liability</h3>
<p>To the fullest extent permitted by law, we shall not be liable...</p>

<h3>6. Governing Law</h3>
<p>These Terms shall be governed by and construed in accordance with...</p>`,
cookie: `<h2>Cookie Policy</h2>
<p>Last updated: [DATE]</p>

<h3>1. What Are Cookies</h3>
<p>Cookies are small text files stored on your device when you visit our website...</p>

<h3>2. Types of Cookies We Use</h3>
<ul>
<li><strong>Essential Cookies:</strong> Required for basic site functionality</li>
<li><strong>Analytics Cookies:</strong> Help us understand how visitors use our site</li>
<li><strong>Marketing Cookies:</strong> Used to deliver relevant advertisements</li>
</ul>

<h3>3. Managing Cookies</h3>
<p>You can control cookies through your browser settings...</p>

<h3>4. Contact Us</h3>
<p>For questions about our Cookie Policy, contact us at...</p>`
};

function insertTemplate(type) {
const content = document.getElementById('content');
const today = new Date().toLocaleDateString();
content.value = templates[type].replace('[DATE]', today);
}

function saveDraft() {
document.getElementById('status').value = 'draft';
document.getElementById('documentForm').dispatchEvent(new Event('submit'));
}

document.getElementById('documentForm').addEventListener('submit', async (e) => {
e.preventDefault();

const data = {
document_type: document.getElementById('documentType').value,
title: document.getElementById('title').value,
description: document.getElementById('description').value || null,
content: document.getElementById('content').value,
status: document.getElementById('status').value,
effective_date: document.getElementById('effectiveDate').value || null,
expiry_date: document.getElementById('expiryDate').value || null
};

try {
const response = await fetch('/api/legal/documents', {
method: 'POST',
headers: { 'Content-Type': 'application/json' },
body: JSON.stringify(data)
});

if (response.ok) {
const doc = await response.json();
window.location = `/suite/legal/${doc.id}`;
} else {
alert('Failed to create document');
}
} catch (e) {
alert('Error: ' + e.message);
}
});
</script>
</body>
</html>"#;
Html(html.to_string())
}
pub fn configure_legal_ui_routes() -> Router<Arc<DbPool>> {
    Router::new()
        .route("/suite/legal", get(handle_legal_list_page))
        .route("/suite/legal/new", get(handle_legal_new_page))
        .route("/suite/legal/:id", get(handle_legal_detail_page))
}
