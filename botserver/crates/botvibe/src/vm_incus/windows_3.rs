//! `vm_incus::windows_3` — split per #1443 (AGENTS.md 450-line rule).

use super::*;

impl VmLifecycle {
    /// Run the project's own app inside the dev container as a REAL process
    /// (visible in the project terminal's `ps`), exposed through a host proxy
    /// device. This is the Linux equivalent of the Windows `deploy_node_files`
    /// flow: workspace files are pushed into `/opt/vibe/app`, node is
    /// installed when missing, the app is started as a systemd service, and a
    /// `vibe-http` proxy device maps `host_port` → 127.0.0.1:3000.
    ///
    /// The entry point is `index.js` when present; otherwise a minimal node
    /// static file server is generated so a node process always runs (the
    /// user's `ps` complaint: the browser showed the app but nothing was
    /// running on the VM).
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn run_dev_app(
        &self,
        name: &str,
        files: &[serde_json::Value],
        host_port: u16,
        database_url: Option<&str>,
    ) -> Result<String, String> {
        self.skip_if_unavailable()?;
        if !self.linux_running(name)? {
            self.linux_start(name)?;
        }
        let temp = std::env::temp_dir().join(format!("vibe-run-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&temp).map_err(|e| format!("create run temp dir: {e}"))?;

        let result = (|| -> Result<String, String> {
            self.incus_run(
                &[
                    "exec".to_string(),
                    name.to_string(),
                    "--".to_string(),
                    "mkdir".to_string(),
                    "-p".to_string(),
                    "/opt/vibe/app".to_string(),
                ],
                30,
            )
            .map_err(|e| format!("prepare app directory: {e}"))?;

            // #1276 — entry resolution (node/python/static precedence) is
            // extracted into `resolve_web_entry` so the clobber rules stay
            // unit-tested without a container.
            let resolved = resolve_web_entry(files);
            let has_node_entry = !resolved.is_python && !resolved.needs_static_fallback;
            let has_python_entry = resolved.is_python;
            // Static fallback server: a node process that serves the workspace
            // files over HTTP. Used when the project has no index.js AND when
            // the real entry is not a web server (a CLI that exits, or a crash)
            // so the browser never opens against a dead port.
            let server_js = concat!(
                "const http=require('http'),fs=require('fs'),path=require('path');\n",
                "const root='/opt/vibe/app';\n",
                "const mime={'.html':'text/html','.css':'text/css','.js':'text/javascript','.mjs':'text/javascript','.json':'application/json','.svg':'image/svg+xml','.png':'image/png','.jpg':'image/jpeg','.gif':'image/gif','.ico':'image/x-icon','.woff2':'font/woff2','.woff':'font/woff','.ttf':'font/ttf','.txt':'text/plain'};\n",
                "http.createServer((req,res)=>{\n",
                "  let p=path.join(root,decodeURIComponent((req.url||'/').split('?')[0]));\n",
                "  if(p.endsWith('/'))p=path.join(p,'index.html');\n",
                "  fs.readFile(p,(e,d)=>{ if(e){\n",
                "    // No web entry point: show a helpful page instead of a dead 404 so\n",
                "    // Run never opens an empty browser for an app-less project.\n",
                "    const ents=fs.existsSync(root)?fs.readdirSync(root).map(f=>'<li>'+f+'</li>').join(''):'<li>(empty workspace)</li>';\n",
                "    const html='<!doctype html><html><head><meta charset=\"utf-8\"><title>No web app yet</title><style>body{font:16px system-ui;background:#0d1117;color:#e6edf3;display:grid;place-items:center;min-height:100vh;margin:0}main{max-width:560px;padding:28px;border:1px solid #30363d;border-radius:16px;background:#161b22}h1{color:#84d669}p{color:#8b949e}code{background:#0d1117;padding:2px 6px;border-radius:6px}ul{color:#8b949e}</style></head><body><main><h1>No web app in this project yet</h1><p>This project has no <code>index.html</code> entry point. Ask the Vibe agent in Chat to build one, then press Run again.</p><ul>'+ents+'</ul></main></body></html>';\n",
                "    res.writeHead(404,{'Content-Type':'text/html; charset=utf-8'});res.end(html);return;} \n",
                "    res.writeHead(200,{'Content-Type':mime[path.extname(p)]||'application/octet-stream'});res.end(d);});\n",
                "}).listen(3000);\n",
            );
            self.push_run_files(name, files, &temp)?;

            // Static apps (pure htmx/html) with no node entrypoint get a
            // generated node server so a node process is what shows up in the
            // terminal's `ps`. When the project ships its own server.js (node)
            // or a python entry it is honored (has_node_entry /
            // has_python_entry), not overwritten.
            if !has_node_entry && !has_python_entry {
                self.push_run_static_server(name, &temp, server_js)?;
            }

            // Pick the web entry + runtime (extracted + unit-tested — #1276).
            // This drives the systemd ExecStart and the runtime bootstrap
            // below.
            let is_python = resolved.is_python;
            let entry = resolved.entry;
            // #1386 — the project's own database URL is injected into the
            // unit so app code reaches its per-environment database through
            // the standard `DATABASE_URL` convention.
            let db_line = database_url
                .map(|url| format!("\nEnvironment=DATABASE_URL={url}"))
                .unwrap_or_default();
            let unit = format!(
                "[Unit]\nDescription=Vibe application\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory=/opt/vibe/app\nEnvironment=PORT=3000{db_line}\nExecStart={} /opt/vibe/app/{entry}\nRestart=always\nRestartSec=2\n\n[Install]\nWantedBy=multi-user.target\n",
                if is_python { "/usr/bin/python3" } else { "/usr/bin/node" }
            );
            self.push_run_unit(name, &temp, &unit)?;

            // Bootstrap the project runtime and install dependencies.
            self.install_run_dependencies(name, files, is_python)?;

            let start: &[&[&str]] = &[
                &["systemctl", "daemon-reload"],
                &["systemctl", "enable", "vibe-app.service"],
                &["systemctl", "restart", "vibe-app.service"],
            ];
            self.run_systemctl(name, "start Vibe application", start)?;

            // Health check: if the project entry is not a web server (a CLI
            // that prints usage and exits, a crash, or a long build step)
            // nothing listens on :3000 and the browser would open against a
            // dead port. Fall back to the generated static server so the
            // browser always shows the workspace and `ps` shows a live node
            // process instead of a crash-loop.
            // The probe must live in a FILE: the command guard rejects shell
            // metacharacters in arguments, so `incus exec -- bash -c "..."`
            // (with `$`, `;`, `>`) is refused and the probe always fails,
            // which wrongly swapped every custom/node app to the static
            // server. A pushed probe script runs with clean arguments.
            let probe_dest = self.push_run_health_probe(name, &temp, is_python)?;
            let probe_interp = if is_python { "python3" } else { "node" };
            if !self.run_is_listening(name, probe_interp, &probe_dest, 20) && has_node_entry {
                log::info!(
                    "Vibe: {name} entry is not a web server (nothing on :3000) — serving workspace statically"
                );
                self.push_run_static_server(name, &temp, server_js)?;
                let db_line = database_url
                    .map(|url| format!("\nEnvironment=DATABASE_URL={url}"))
                    .unwrap_or_default();
                let unit = format!(
                    "[Unit]\nDescription=Vibe application (static)\nAfter=network.target\n\n[Service]\nType=simple\nWorkingDirectory=/opt/vibe/app\nEnvironment=PORT=3000{db_line}\nExecStart=/usr/bin/node /opt/vibe/app/server.js\nRestart=always\nRestartSec=2\n\n[Install]\nWantedBy=multi-user.target\n"
                );
                self.push_run_unit(name, &temp, &unit)?;
                let restart: &[&[&str]] = &[
                    &["systemctl", "daemon-reload"],
                    &["systemctl", "restart", "vibe-app.service"],
                ];
                self.run_systemctl(name, "start static fallback", restart)?;
                if !self.run_is_listening(name, probe_interp, &probe_dest, 15) {
                    log::warn!("Vibe: {name} static fallback is not serving on :3000");
                }
            }

            // host_port == 0: prod publish path — no proxy device is attached
            // (the app is served through the Caddy domain route → container
            // IP:3000). Attaching a device with the previous default of 80
            // failed with `bind: address already in use` on prod.
            self.attach_run_proxy_device(name, host_port)?;
            if host_port == 0 {
                Ok("http://localhost:3000".to_string())
            } else if host_port == 80 {
                Ok("http://localhost".to_string())
            } else {
                Ok(format!("http://localhost:{host_port}"))
            }
        })();

        if let Err(e) = std::fs::remove_dir_all(&temp) {
            log::warn!("Vibe: failed to remove run temp dir: {e}");
        }
        result
    }

    /// Push the project's files into `/opt/vibe/app` inside the dev
    /// container. Relative paths are validated so a crafted name cannot
    /// escape the app directory.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn push_run_files(
        &self,
        name: &str,
        files: &[serde_json::Value],
        temp: &std::path::Path,
    ) -> Result<(), String> {
        for file in files {
            let rel = file
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "run file is missing path".to_string())?;
            let rel_path = std::path::Path::new(rel);
            if rel_path.is_absolute()
                || rel_path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
            {
                return Err(format!("invalid run path '{rel}'"));
            }
            let bytes = file
                .get("content")
                .and_then(|v| v.as_array())
                .ok_or_else(|| format!("run file '{rel}' has invalid content"))?
                .iter()
                .map(|v| {
                    v.as_u64()
                        .filter(|n| *n <= u8::MAX as u64)
                        .map(|n| n as u8)
                        .ok_or_else(|| format!("run file '{rel}' has invalid byte"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let source = temp.join(rel_path);
            if let Some(parent) = source.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("create run path for '{rel}': {e}"))?;
            }
            std::fs::write(&source, bytes).map_err(|e| format!("write run file '{rel}': {e}"))?;
            let destination = format!("{name}/opt/vibe/app/{}", rel.replace('\\', "/"));
            self.incus_run(
                &[
                    "file".to_string(),
                    "push".to_string(),
                    source.to_string_lossy().into_owned(),
                    destination,
                    "--create-dirs".to_string(),
                ],
                60,
            )
            .map_err(|e| format!("push run file '{rel}': {e}"))?;
        }
        Ok(())
    }

    /// Push the generated static file server as `/opt/vibe/app/server.js`.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn push_run_static_server(
        &self,
        name: &str,
        temp: &std::path::Path,
        server_js: &str,
    ) -> Result<(), String> {
        let server_path = temp.join("server.js");
        std::fs::write(&server_path, server_js).map_err(|e| format!("write static server: {e}"))?;
        self.incus_run(
            &[
                "file".to_string(),
                "push".to_string(),
                server_path.to_string_lossy().into_owned(),
                format!("{name}/opt/vibe/app/server.js"),
                "--create-dirs".to_string(),
            ],
            60,
        )
        .map_err(|e| format!("push static server: {e}"))?;
        Ok(())
    }

    /// Write a systemd unit locally and push it as `vibe-app.service`.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn push_run_unit(
        &self,
        name: &str,
        temp: &std::path::Path,
        unit: &str,
    ) -> Result<(), String> {
        let unit_path = temp.join("vibe-app.service");
        std::fs::write(&unit_path, unit).map_err(|e| format!("write service unit: {e}"))?;
        self.incus_run(
            &[
                "file".to_string(),
                "push".to_string(),
                unit_path.to_string_lossy().into_owned(),
                format!("{name}/etc/systemd/system/vibe-app.service"),
                "--create-dirs".to_string(),
            ],
            60,
        )
        .map_err(|e| format!("push service unit: {e}"))?;
        Ok(())
    }

    /// Attach (replacing any existing) the `vibe-http` proxy device mapping
    /// `host_port` → 127.0.0.1:3000. `host_port == 0` is the prod publish
    /// path, where the app is reached through the Caddy domain route and no
    /// device is attached.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn attach_run_proxy_device(&self, name: &str, host_port: u16) -> Result<(), String> {
        let devices = self
            .incus_run(
                &[
                    "config".to_string(),
                    "device".to_string(),
                    "show".to_string(),
                    name.to_string(),
                ],
                30,
            )
            .map_err(|e| format!("inspect proxy device: {e}"))?;
        if devices.stdout.contains("vibe-http:") {
            self.incus_run(
                &[
                    "config".to_string(),
                    "device".to_string(),
                    "remove".to_string(),
                    name.to_string(),
                    "vibe-http".to_string(),
                ],
                30,
            )
            .map_err(|e| format!("replace proxy device: {e}"))?;
        }
        if host_port == 0 {
            return Ok(());
        }
        self.incus_run(
            &[
                "config".to_string(),
                "device".to_string(),
                "add".to_string(),
                name.to_string(),
                "vibe-http".to_string(),
                "proxy".to_string(),
                format!("listen=tcp:0.0.0.0:{host_port}"),
                "connect=tcp:127.0.0.1:3000".to_string(),
            ],
            60,
        )
        .map_err(|e| format!("expose application port: {e}"))?;
        Ok(())
    }
}
