//! `vm_incus::windows_4` — split from `windows_3` per #1492 (AGENTS.md
//! 450-line rule). Runtime bootstrap and health-probe helpers used by the
//! Linux `run_dev_app` flow.

use super::*;

impl VmLifecycle {
    /// Install the project runtime and dependencies inside the dev container.
    ///
    /// Python apps check for `python3` (and pip so `requirements.txt` deps
    /// resolve before the service starts); node apps check for
    /// `nodejs`/`npm`; static fallbacks run with node.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn install_run_dependencies(
        &self,
        name: &str,
        files: &[serde_json::Value],
        is_python: bool,
    ) -> Result<(), String> {
        if is_python {
            if self
                .incus_run(
                    &[
                        "exec".to_string(),
                        name.to_string(),
                        "--".to_string(),
                        "python3".to_string(),
                        "--version".to_string(),
                    ],
                    30,
                )
                .is_err()
            {
                // Fresh containers have no TTY; debconf's Dialog frontend
                // aborts with exit 100 unless the frontend is noninteractive.
                for command in [
                    vec!["apt-get", "update"],
                    vec!["apt-get", "install", "-y", "python3", "python3-pip"],
                ] {
                    let mut args = vec!["exec".to_string(), name.to_string()];
                    args.push("--env".to_string());
                    args.push("DEBIAN_FRONTEND=noninteractive".to_string());
                    args.push("--".to_string());
                    args.extend(command.into_iter().map(str::to_string));
                    self.incus_run(&args, 300)
                        .map_err(|e| format!("install Python runtime: {e}"))?;
                }
            }
            // Resolve project dependencies before the service starts.
            if files.iter().any(|f| {
                let p = f.get("path").and_then(|v| v.as_str()).unwrap_or("");
                p == "requirements.txt" || p.ends_with("/requirements.txt")
            }) {
                let mut args = vec![
                    "exec".to_string(),
                    name.to_string(),
                    "--".to_string(),
                    "python3".to_string(),
                    "-m".to_string(),
                    "pip".to_string(),
                    "install".to_string(),
                    "-r".to_string(),
                    "/opt/vibe/app/requirements.txt".to_string(),
                ];
                // Some base images are externally-managed (PEP 668) and
                // refuse system installs; --break-system-packages keeps a
                // few-line python app running on Ubuntu 24.04 VMs.
                args.push("--break-system-packages".to_string());
                let _ = self.incus_run(&args, 300);
            }
        } else if self
            .incus_run(
                &[
                    "exec".to_string(),
                    name.to_string(),
                    "--".to_string(),
                    "node".to_string(),
                    "--version".to_string(),
                ],
                30,
            )
            .is_err()
        {
            // Fresh containers have no TTY; debconf's Dialog frontend
            // aborts with exit 100 unless the frontend is noninteractive.
            for command in [
                vec!["apt-get", "update"],
                vec!["apt-get", "install", "-y", "nodejs", "npm"],
            ] {
                let mut args = vec!["exec".to_string(), name.to_string()];
                args.push("--env".to_string());
                args.push("DEBIAN_FRONTEND=noninteractive".to_string());
                args.push("--".to_string());
                args.extend(command.into_iter().map(str::to_string));
                self.incus_run(&args, 300)
                    .map_err(|e| format!("install Node runtime: {e}"))?;
            }
        }

        // Install project dependencies when a package.json exists (the same
        // trigger the python branch uses for requirements.txt) so a Run serves
        // the real app even when the agent never ran npm install itself.
        // Tolerated failure: the static fallback probe rescues projects whose
        // dependencies cannot resolve.
        // #1273 — skipped when both manifests are byte-identical to the
        // previous Run: the hash file written after a successful install turns
        // every subsequent Run into a no-op instead of re-paying the npm tax
        // on slow networks.
        if files
            .iter()
            .any(|f| f.get("path").and_then(|v| v.as_str()) == Some("package.json"))
        {
            let mut hasher_input = Vec::new();
            for manifest in ["package.json", "package-lock.json"] {
                if let Some(f) = files
                    .iter()
                    .find(|f| f.get("path").and_then(|v| v.as_str()) == Some(manifest))
                {
                    if let Some(content) = f.get("content").and_then(|v| v.as_array()) {
                        for b in content {
                            hasher_input.push(b.as_u64().unwrap_or(0) as u8);
                        }
                    }
                }
                hasher_input.push(b'\n');
            }
            let deps_hash = format!("{:x}", Sha256::digest(&hasher_input));
            let check = self.incus_run(
                &[
                    "exec".to_string(),
                    name.to_string(),
                    "--".to_string(),
                    "cat".to_string(),
                    "/opt/vibe/app/.vibe-deps-hash".to_string(),
                ],
                15,
            );
            let unchanged = check
                .map(|out| out.stdout.trim() == deps_hash)
                .unwrap_or(false);
            if unchanged {
                log::info!("Vibe: {name} dependencies unchanged — skipping npm install");
            } else {
                let args = vec![
                    "exec".to_string(),
                    name.to_string(),
                    "--env".to_string(),
                    "DEBIAN_FRONTEND=noninteractive".to_string(),
                    "--".to_string(),
                    "npm".to_string(),
                    "install".to_string(),
                    "--prefix".to_string(),
                    "/opt/vibe/app".to_string(),
                    "--no-audit".to_string(),
                    "--no-fund".to_string(),
                ];
                if let Err(e) = self.incus_run(&args, 300) {
                    log::warn!("Vibe: {name} npm install failed: {e}");
                } else {
                    // Record the hash only after a successful install so a
                    // failed attempt retries on the next Run.
                    let _ = self.incus_run(
                        &[
                            "exec".to_string(),
                            name.to_string(),
                            "--".to_string(),
                            "sh".to_string(),
                            "-c".to_string(),
                            format!("printf %s {} > /opt/vibe/app/.vibe-deps-hash", deps_hash),
                        ],
                        15,
                    );
                }
            }
        }
        Ok(())
    }

    /// Push the health-check probe into the container and return the
    /// in-container path it was written to.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn push_run_health_probe(
        &self,
        name: &str,
        temp: &std::path::Path,
        is_python: bool,
    ) -> Result<String, String> {
        let probe_file = if is_python {
            "healthcheck.py"
        } else {
            "healthcheck.js"
        };
        let probe_path = temp.join(format!("vibe-{probe_file}"));
        let probe_src = if is_python {
            HEALTH_PROBE_PYTHON
        } else {
            HEALTH_PROBE_JS
        };
        std::fs::write(&probe_path, probe_src).map_err(|e| format!("write health probe: {e}"))?;
        self.incus_run(
            &[
                "file".to_string(),
                "push".to_string(),
                probe_path.to_string_lossy().into_owned(),
                format!("{name}/opt/vibe/{probe_file}"),
                "--create-dirs".to_string(),
            ],
            60,
        )
        .map_err(|e| format!("push health probe: {e}"))?;
        Ok(format!("/opt/vibe/{probe_file}"))
    }

    /// Run the pushed health probe; true when something listens on :3000.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn run_is_listening(
        &self,
        name: &str,
        interpreter: &str,
        probe: &str,
        attempts: u32,
    ) -> bool {
        self.incus_run(
            &[
                "exec".to_string(),
                name.to_string(),
                "--".to_string(),
                interpreter.to_string(),
                probe.to_string(),
                attempts.to_string(),
            ],
            60,
        )
        .is_ok()
    }

    /// Run a sequence of `systemctl` commands inside the dev container.
    #[cfg(not(target_os = "windows"))]
    pub(crate) fn run_systemctl(
        &self,
        name: &str,
        context: &str,
        commands: &[&[&str]],
    ) -> Result<(), String> {
        for command in commands {
            let mut args = vec!["exec".to_string(), name.to_string(), "--".to_string()];
            args.extend(command.iter().map(|s| (*s).to_string()));
            self.incus_run(&args, 60)
                .map_err(|e| format!("{context}: {e}"))?;
        }
        Ok(())
    }
}
