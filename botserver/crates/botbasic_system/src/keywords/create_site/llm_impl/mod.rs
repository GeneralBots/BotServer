pub(crate) use botcore::shared::state::AppState;
pub(crate) use botcore::shared::UserSession;
pub(crate) use log::{debug, info, warn};
pub(crate) use rhai::{Dynamic, Engine};
pub(crate) use serde_json::json;
pub(crate) use std::error::Error;
pub(crate) use std::fs;
pub(crate) use std::io::Read;
pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::Arc;

struct SiteCreationParams {
    alias: Dynamic,
    template_dir: Dynamic,
    prompt: Dynamic,
}

#[cfg(not(feature = "vibe"))]
const DEFAULT_STYLES: &str = r"
:root {
  --primary: #0ea5e9;
  --success: #22c55e;
  --warning: #f59e0b;
  --danger: #ef4444;
  --bg: #ffffff;
  --text: #1e293b;
  --border: #e2e8f0;
  --radius: 8px;
}

@media (prefers-color-scheme: dark) {
  :root {
--bg: #0f172a;
--text: #f1f5f9;
--border: #334155;
  }
}

* { box-sizing: border-box; margin: 0; padding: 0; }

body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: var(--bg); color: var(--text); line-height: 1.5; }

header { padding: 1rem 2rem; border-bottom: 1px solid var(--border); }

main { padding: 2rem; max-width: 1200px; margin: 0 auto; }

h1, h2, h3 { margin-bottom: 1rem; }

input, select, textarea { padding: 0.5rem 1rem; border: 1px solid var(--border); border-radius: var(--radius); background: var(--bg); color: var(--text); font-size: 1rem; }

input:focus, select:focus, textarea:focus { outline: none; border-color: var(--primary); }

button { padding: 0.5rem 1rem; background: var(--primary); color: white; border: none; border-radius: var(--radius); cursor: pointer; font-size: 1rem; }

button:hover { opacity: 0.9; }

form { display: flex; gap: 0.5rem; margin: 1rem 0; }

table { width: 100%; border-collapse: collapse; }

th, td { padding: 0.75rem; text-align: left; border-bottom: 1px solid var(--border); }

.htmx-indicator { opacity: 0; transition: opacity 0.2s; }
.htmx-request .htmx-indicator { opacity: 1; }
";

#[cfg(not(feature = "vibe"))]
const DEFAULT_APP_JS: &str = r"

function toast(message, type = 'info') {
  const el = document.createElement('div');
  el.className = 'toast toast-' + type;
  el.textContent = message;
  el.style.cssText = 'position:fixed;bottom:20px;right:20px;padding:1rem;background:#333;color:#fff;border-radius:8px;z-index:9999;';
  document.body.appendChild(el);
  setTimeout(() => el.remove(), 3000);
}


document.body.addEventListener('htmx:afterSwap', function(e) {
  console.log('Data updated:', e.detail.target.id);
});

document.body.addEventListener('htmx:responseError', function(e) {
  toast('Error: ' + (e.detail.xhr.responseText || 'Request failed'), 'error');
});


function openModal(id) {
  document.getElementById(id)?.classList.add('active');
}

function closeModal(id) {
  document.getElementById(id)?.classList.remove('active');
}
";

mod part1;
mod part2;
mod part3;
mod part4;

pub use part1::*;
use part2::*;
use part3::*;
pub use part4::*;
