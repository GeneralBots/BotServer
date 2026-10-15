//! Built-in project templates — the Vibe codebase owns the starter content it
//! seeds into a fresh project workspace, so nothing depends on a pre-seeded
//! external tree (the old `/tmp/vibe-workspaces/calculator` fixture).
//!
//! A "calculator" project gets a working arithmetic service: a safe, eval-free
//! expression evaluator (`calc.js`), an HTTP service (`index.js`), a test
//! suite (`test.js`) and a `package.json`. Any other software project gets a
//! minimal README starter. Seeding never clobbers agent output: an already
//! non-empty workspace is left untouched.

const CALCULATOR_CALC_JS: &str = r#"'use strict';
// Safe arithmetic evaluator for the Vibe calculator template.
// Handles + - * / and parentheses; parses without eval().
function tokenize(input) {
  const tokens = [];
  const re = /\s*(\d+(?:\.\d+)?|[-+*/()])/g;
  let m;
  let last = 0;
  while ((m = re.exec(input))) {
    if (m.index !== last) throw new Error('invalid character at position ' + last);
    last = re.lastIndex;
    const tok = m[1];
    tokens.push(/^[-+*/()]$/.test(tok) ? tok : parseFloat(tok));
  }
  if (last !== input.length) throw new Error('invalid character at position ' + last);
  return tokens;
}

function evaluate(expr) {
  const tokens = tokenize(String(expr));
  if (tokens.length === 0) return 0;
  let i = 0;
  const peek = () => tokens[i];
  const next = () => tokens[i++];

  function parseExpr() {
    let value = parseTerm();
    while (peek() === '+' || peek() === '-') {
      const op = next();
      const rhs = parseTerm();
      value = op === '+' ? value + rhs : value - rhs;
    }
    return value;
  }
  function parseTerm() {
    let value = parseFactor();
    while (peek() === '*' || peek() === '/') {
      const op = next();
      const rhs = parseFactor();
      if (op === '/' && rhs === 0) throw new Error('division by zero');
      value = op === '*' ? value * rhs : value / rhs;
    }
    return value;
  }
  function parseFactor() {
    const tok = peek();
    if (tok === '(') {
      next();
      const v = parseExpr();
      if (next() !== ')') throw new Error('unbalanced parentheses');
      return v;
    }
    if (tok === '-') { next(); return -parseFactor(); }
    if (typeof tok === 'number') { next(); return tok; }
    throw new Error('unexpected token: ' + tok);
  }

  const result = parseExpr();
  if (i !== tokens.length) throw new Error('unexpected trailing input');
  return result;
}

module.exports = { evaluate };
"#;

const CALCULATOR_INDEX_JS: &str = r#"'use strict';
const http = require('http');
const { evaluate } = require('./calc');

const port = Number(process.env.PORT || 3000);

function respond(res, code, body) {
  res.writeHead(code, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(body));
}

const page = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Vibe Calculator</title><style>
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:#0d1117;color:#e6edf3;font:16px system-ui,sans-serif}
main{width:min(92vw,420px);padding:28px;border:1px solid #30363d;border-radius:18px;background:#161b22;box-shadow:0 24px 70px #0008}
h1{margin:0 0 6px;color:#84d669}p{color:#8b949e;margin:0 0 20px}form{display:flex;gap:10px}input{min-width:0;flex:1;padding:14px;border:1px solid #484f58;border-radius:10px;background:#0d1117;color:#fff;font-size:18px}
button{border:0;border-radius:10px;padding:0 18px;background:#84d669;color:#10220b;font-weight:800;cursor:pointer}output{display:block;min-height:72px;margin-top:18px;padding:18px;border-radius:12px;background:#0d1117;font-size:28px;font-weight:800}
</style></head><body><main><h1>Vibe Calculator</h1><p>Published locally with Windows + WSL + Incus</p>
<form id="calc"><input id="expr" aria-label="Expression" value="(12 + 8) * 3" autofocus><button>Calculate</button></form><output id="result">Ready</output>
<script>document.getElementById('calc').addEventListener('submit',async e=>{e.preventDefault();const q=document.getElementById('expr').value;const out=document.getElementById('result');out.textContent='…';try{const r=await fetch('/api/calculate?expr='+encodeURIComponent(q));const d=await r.json();out.textContent=r.ok?d.result:d.error}catch(err){out.textContent=err.message}})</script>
</main></body></html>`;

const server = http.createServer((req, res) => {
  const url = new URL(req.url, 'http://localhost');
  if (url.pathname === '/health') return respond(res, 200, { status: 'ok' });
  if (url.pathname === '/' && req.method === 'GET' && !url.searchParams.has('expr')) {
    res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
    return res.end(page);
  }
  if (!['/', '/api/calculate'].includes(url.pathname) || req.method !== 'GET') return respond(res, 404, { error: 'not found' });
  const expr = url.searchParams.get('expr') || '';
  try {
    const result = evaluate(expr);
    respond(res, 200, { expr, result });
  } catch (err) {
    respond(res, 400, { error: err.message });
  }
});

server.listen(port, () => console.log('calculator listening on http://0.0.0.0:' + port));
"#;

const CALCULATOR_TEST_JS: &str = r#"'use strict';
const assert = require('assert');
const { evaluate } = require('./calc');

const cases = [
  ['2+3', 5],
  ['10-4', 6],
  ['3*4', 12],
  ['20/5', 4],
  ['2+3*4', 14],
  ['(2+3)*4', 20],
  ['-5+2', -3],
  ['1.5*2', 3],
  ['100/0', null], // must throw division by zero
];

let passed = 0;
for (const [expr, want] of cases) {
  if (want === null) {
    assert.throws(() => evaluate(expr), /division by zero/, expr + ' must throw');
  } else {
    assert.strictEqual(evaluate(expr), want, expr + ' => ' + want);
  }
  passed++;
}
console.log('all ' + passed + ' tests passed');
"#;

const CALCULATOR_PACKAGE_JSON: &str = r#"{
  "name": "calculator",
  "version": "1.0.0",
  "description": "Arithmetic calculator service with a safe, eval-free evaluator",
  "main": "index.js",
  "scripts": {
    "start": "node index.js",
    "test": "node test.js"
  },
  "license": "MIT"
}
"#;

const CALCULATOR_README: &str = r#"# Calculator

Arithmetic calculator service owned by Vibe's built-in project templates.

- `calc.js` — safe, eval-free expression evaluator (+, -, *, /, parentheses)
- `index.js` — HTTP service: `GET /?expr=2%2B3*4`
- `test.js` — test suite (`npm test`)
- `/health` — liveness probe

Run locally: `npm start` (PORT env, default 3000).
"#;

const STARTER_README: &str = r#"# Project

This project was scaffolded by Vibe's built-in project template.

Use the file tools to add source code, then `project_test` to verify and
`publish/project` to deploy it to an environment.
"#;

/// Starter page for Website (htmx) projects: a self-contained static page
/// (inline CSS/JS — no CDN, per project rules) so Run renders a real app
/// instead of the "No web app yet" fallback. htmx is the declared framework
/// for website projects; the template stays dependency-free so it runs in
/// the VM with zero install steps.
const WEBSITE_INDEX_HTML: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>Vibe Website</title><style>
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:#0d1117;color:#e6edf3;font:16px system-ui,sans-serif}
main{width:min(92vw,520px);padding:32px;border:1px solid #30363d;border-radius:18px;background:#161b22;box-shadow:0 24px 70px #0008;text-align:center}
h1{margin:0 0 8px;color:#84d669;font-size:28px}p{color:#8b949e;margin:0 0 24px;line-height:1.6}
.btn{display:inline-block;border:0;border-radius:10px;padding:12px 22px;background:#84d669;color:#10220b;font-weight:800;cursor:pointer;text-decoration:none;margin:4px}
.btn:hover{filter:brightness(1.1)}#demo{margin-top:22px;padding:16px;border-radius:12px;background:#0d1117;font-size:15px;color:#e6edf3;display:none}
</style></head><body><main>
<h1>Vibe Website</h1>
<p>This is the starter page for <strong>Website</strong> projects. Edit <code>index.html</code> in the Editor to build your site, then press <strong>Run</strong> again.</p>
<button class="btn" id="demoBtn">Show htmx-style demo</button>
<div id="demo"></div>
<script>
document.getElementById('demoBtn').addEventListener('click', function () {
  var d = document.getElementById('demo');
  d.style.display = 'block';
  d.textContent = 'Server time: ' + new Date().toLocaleTimeString() + ' — served by Vibe on ' + location.hostname;
});
</script>
</main></body></html>
"#;

/// Starter service for Custom/python projects: a dependency-free HTTP server
/// (stdlib only) so Run starts a real python process and serves a page. The
/// VM runs it as `/usr/bin/python3 app.py` with `PORT=3000`.
const PYTHON_APP_PY: &str = r#"#!/usr/bin/env python3
"""Vibe python starter — stdlib HTTP server, no external dependencies."""
import os
from http.server import BaseHTTPRequestHandler, HTTPServer

PORT = int(os.environ.get("PORT", "3000"))

PAGE = """<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Vibe Python</title><style>
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:#0d1117;color:#e6edf3;font:16px system-ui,sans-serif}
main{width:min(92vw,520px);padding:32px;border:1px solid #30363d;border-radius:18px;background:#161b22;box-shadow:0 24px 70px #0008;text-align:center}
h1{margin:0 0 8px;color:#84d669}p{color:#8b949e;margin:0 0 20px}
code{background:#0d1117;padding:2px 8px;border-radius:6px;color:#e6edf3}
</style></head><body><main>
<h1>Vibe Python</h1>
<p>This is the starter for <strong>Custom / python</strong> projects. Edit <code>app.py</code> in the Editor to build your service, then press <strong>Run</strong> again.</p>
</main></body></html>
"""


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            body = b'{"status":"ok"}'
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        body = PAGE.encode("utf-8")
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


if __name__ == "__main__":
    print(f"python starter listening on http://0.0.0.0:{PORT}")
    HTTPServer(("0.0.0.0", PORT), Handler).serve_forever()
"#;

/// Starter service for Custom/node projects that are not calculator-style:
/// a minimal stdlib http server so Run starts a real node process and serves
/// a page (the VM runs it as `/usr/bin/node index.js` with `PORT=3000`).
const NODE_INDEX_JS: &str = r#"'use strict';
const http = require('http');
const port = Number(process.env.PORT || 3000);

const page = `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>Vibe Node</title><style>
*{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:#0d1117;color:#e6edf3;font:16px system-ui,sans-serif}
main{width:min(92vw,520px);padding:32px;border:1px solid #30363d;border-radius:18px;background:#161b22;box-shadow:0 24px 70px #0008;text-align:center}
h1{margin:0 0 8px;color:#84d669}p{color:#8b949e;margin:0 0 20px}
code{background:#0d1117;padding:2px 8px;border-radius:6px;color:#e6edf3}
</style></head><body><main>
<h1>Vibe Node</h1>
<p>This is the starter for <strong>Custom / node</strong> projects. Edit <code>index.js</code> in the Editor to build your service, then press <strong>Run</strong> again.</p>
</main></body></html>`;

const server = http.createServer((req, res) => {
  if (req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end('{"status":"ok"}');
  }
  res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
  res.end(page);
});

server.listen(port, () => console.log('node starter listening on http://0.0.0.0:' + port));
"#;

/// True when the project name asks for a calculator-style app.
pub fn is_calculator_project(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("calculator") || n.contains("calc")
}

/// Pick the built-in starter matching a project's kind and framework, then
/// seed it into the workspace at `workspace_root()/{key}`. Calculator-style
/// names win (regardless of type); otherwise the matrix is:
///   website + htmx  -> `index.html` (self-contained static page)
///   custom + python -> `app.py`    (stdlib http server)
///   custom + node   -> `index.js`  (stdlib http server)
///   bot / others    -> README starter only
/// No-op when the workspace already has files (the agent may have started
/// working).
pub fn seed_project_workspace(
    key: &str,
    project_name: &str,
    project_type: &str,
    framework: Option<&str>,
) -> Result<(), String> {
    let key = crate::harness::sanitize_project_id(key)?;
    let dir = crate::harness::workspace_root().join(&key);
    if dir.is_dir() {
        let mut entries =
            std::fs::read_dir(&dir).map_err(|e| format!("read workspace {key}: {e}"))?;
        if entries.next().is_some() {
            return Ok(());
        }
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("create workspace {key}: {e}"))?;

    let calculator = is_calculator_project(project_name);
    let fw = framework.unwrap_or("").to_lowercase();
    let template = if calculator {
        crate::harness::write_rel_file(&key, "calc.js", CALCULATOR_CALC_JS.as_bytes())?;
        crate::harness::write_rel_file(&key, "index.js", CALCULATOR_INDEX_JS.as_bytes())?;
        crate::harness::write_rel_file(&key, "test.js", CALCULATOR_TEST_JS.as_bytes())?;
        crate::harness::write_rel_file(&key, "package.json", CALCULATOR_PACKAGE_JSON.as_bytes())?;
        "calculator"
    } else if project_type.eq_ignore_ascii_case("website") {
        crate::harness::write_rel_file(&key, "index.html", WEBSITE_INDEX_HTML.as_bytes())?;
        "website"
    } else if fw == "python" {
        crate::harness::write_rel_file(&key, "app.py", PYTHON_APP_PY.as_bytes())?;
        "python"
    } else if fw == "node" {
        crate::harness::write_rel_file(&key, "index.js", NODE_INDEX_JS.as_bytes())?;
        "node"
    } else {
        "starter"
    };
    let readme = if calculator {
        CALCULATOR_README
    } else {
        STARTER_README
    };
    crate::harness::write_rel_file(&key, "README.md", readme.as_bytes())?;

    log::info!("Vibe: seeded workspace '{key}' with {template} template");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculator_detection() {
        assert!(is_calculator_project("calculator"));
        assert!(is_calculator_project("My Calculator App"));
        assert!(is_calculator_project("calc"));
        assert!(!is_calculator_project("landing-page"));
        assert!(!is_calculator_project(""));
    }

    #[tokio::test]
    async fn seed_seeds_calculator_and_never_clobbers() {
        let _guard = crate::harness::WORKSPACE_ENV_LOCK.lock().await;
        let previous = std::env::var_os("VIBE_WORKSPACE_ROOT");
        let tmp =
            std::env::temp_dir().join(format!("vibe-templates-test-{}", uuid::Uuid::new_v4()));
        std::env::set_var("VIBE_WORKSPACE_ROOT", &tmp);
        let key = "calculator";

        seed_project_workspace(key, "Calculator", "apps", Some("node")).expect("seed");
        let entries = crate::harness::list_rel(key, "", 0).expect("list");
        for want in [
            "calc.js",
            "index.js",
            "test.js",
            "package.json",
            "README.md",
        ] {
            assert!(
                entries.iter().any(|e| e == want),
                "missing {want}: {entries:?}"
            );
        }
        let calc =
            crate::harness::read_rel_file(key, "calc.js", 1024 * 1024).expect("read calc.js");
        let calc = String::from_utf8(calc).expect("utf8");
        assert!(calc.contains("evaluate"), "calc.js must export evaluate");

        // Second seed is a no-op (workspace already populated).
        seed_project_workspace(key, "Calculator", "apps", Some("node")).expect("re-seed");
        let after = crate::harness::list_rel(key, "", 0).expect("list after");
        assert_eq!(after.len(), entries.len());

        let _ = std::fs::remove_dir_all(&tmp);
        crate::harness::restore_workspace_root(previous);
    }

    #[tokio::test]
    async fn seed_starter_for_non_calculator() {
        let _guard = crate::harness::WORKSPACE_ENV_LOCK.lock().await;
        let previous = std::env::var_os("VIBE_WORKSPACE_ROOT");
        let tmp =
            std::env::temp_dir().join(format!("vibe-templates-test-{}", uuid::Uuid::new_v4()));
        std::env::set_var("VIBE_WORKSPACE_ROOT", &tmp);
        let key = "landing";

        seed_project_workspace(key, "Landing Page", "website", Some("htmx")).expect("seed");
        let entries = crate::harness::list_rel(key, "", 0).expect("list");
        assert!(entries.iter().any(|e| e == "index.html"));
        assert!(entries.iter().any(|e| e == "README.md"));
        assert!(!entries.iter().any(|e| e == "calc.js"));

        // python framework seeds app.py; node framework seeds index.js
        // (each on its own fresh workspace — seeding never clobbers).
        let py_key = "py-app";
        seed_project_workspace(py_key, "Py App", "apps", Some("python")).expect("seed2");
        let entries = crate::harness::list_rel(py_key, "", 0).expect("list after");
        assert!(entries.iter().any(|e| e == "app.py"), "{entries:?}");
        let node_key = "node-app";
        seed_project_workspace(node_key, "Node App", "apps", Some("node")).expect("seed3");
        let entries = crate::harness::list_rel(node_key, "", 0).expect("list after2");
        assert!(entries.iter().any(|e| e == "index.js"), "{entries:?}");

        let _ = std::fs::remove_dir_all(&tmp);
        crate::harness::restore_workspace_root(previous);
    }
}
