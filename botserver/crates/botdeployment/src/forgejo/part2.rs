use super::*;

impl ForgejoClient {
    async fn push_workflow_preserving(
        &self,
        repo_url: &str,
        app: &GeneratedApp,
    ) -> Result<(), DeploymentError> {
        let temp_dir = app.temp_dir()?;
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)
                .map_err(|e| DeploymentError::GitError(format!("Failed to clean temp dir: {}", e)))?;
        }
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| DeploymentError::GitError(format!("Failed to create temp dir: {}", e)))?;

        let auth_url = self.add_token_to_url(repo_url);
        let git = |args: &[&str]| -> Result<String, DeploymentError> {
            let output = std::process::Command::new("git")
                .current_dir(&temp_dir)
                .args(args)
                .output()
                .map_err(|e| DeploymentError::GitError(format!("git {}: {e}", args[0])))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(DeploymentError::GitError(format!(
                    "git {} failed: {}",
                    args[0],
                    stderr.trim().lines().last().unwrap_or("unknown")
                )));
            }
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        };

        // Clone the existing repo (empty remotes clone cleanly; the working
        // tree is then populated by the file writes below).
        let clone_output = std::process::Command::new("git")
            .current_dir(
                temp_dir
                    .parent()
                    .ok_or_else(|| DeploymentError::GitError("temp dir has no parent".to_string()))?,
            )
            .args(["clone", "--quiet", &auth_url, &temp_dir.to_string_lossy()])
            .output()
            .map_err(|e| DeploymentError::GitError(format!("git clone: {e}")))?;
        if !clone_output.status.success() {
            let stderr = String::from_utf8_lossy(&clone_output.stderr);
            return Err(DeploymentError::GitError(format!(
                "git clone failed: {}",
                stderr.trim().lines().last().unwrap_or("unknown")
            )));
        }

        // Ensure we are on `main` even when the clone produced no checkout
        // (empty remote) or checked out a different default branch.
        git(&["checkout", "-q", "-B", "main"])?;

        for file in &app.files {
            let file_path = temp_dir.join(&file.path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| DeploymentError::GitError(format!("Failed to create parent dir: {}", e)))?;
            }
            std::fs::write(&file_path, &file.content)
                .map_err(|e| DeploymentError::GitError(format!("Failed to write file: {}", e)))?;
        }

        git(&["add", "-A"])?;
        // The workflow may already exist and be identical (re-deploy after a
        // rebase brought it in): a "nothing to commit" is a no-op, not an
        // error — the push below then just confirms everything is in sync.
        let commit_out = std::process::Command::new("git")
            .current_dir(&temp_dir)
            .args(["-c", "user.name=GB Deployer", "-c", "user.email=deployer@generalbots.com",
                   "commit", "-m", &format!("Add CI/CD workflow: {}", app.description)])
            .output()
            .map_err(|e| DeploymentError::GitError(format!("git commit: {e}")))?;
        let commit_text = format!(
            "{} {}",
            String::from_utf8_lossy(&commit_out.stdout),
            String::from_utf8_lossy(&commit_out.stderr)
        );
        if !commit_out.status.success()
            && !(commit_text.contains("nothing to commit")
                || commit_text.contains("no changes added to commit"))
        {
            let stderr = String::from_utf8_lossy(&commit_out.stderr);
            return Err(DeploymentError::GitError(format!(
                "git commit failed: {}",
                stderr.trim().lines().last().unwrap_or("unknown")
            )));
        }
        git(&["push", &auth_url, "HEAD:main"])?;

        Ok(())
    }

    pub(super) fn add_token_to_url(&self, url: &str) -> String {
        // Forgejo personal access tokens authenticate over HTTP(S) via
        // `token@host`; the self-hosted ALM serves plain `http://` clone
        // URLs, so both schemes must embed the token or the push fails
        // with "could not read Username for ..." (no auth at all).
        if let Some(rest) = url.strip_prefix("https://") {
            format!("https://{}@{}", self.token, rest)
        } else if let Some(rest) = url.strip_prefix("http://") {
            format!("http://{}@{}", self.token, rest)
        } else {
            url.to_string()
        }
    }

    pub(super) fn generate_app_workflow(&self, project_type: &ProjectType, _deploy_target: &DeployTarget, environment: &DeploymentEnvironment) -> String {
        let env_name = environment.to_string();
        let (framework, node_version, build_command, output_dir) = match project_type {
            ProjectType::App { framework, node_version, build_command, output_directory } => (
                framework.clone(),
                node_version.clone().unwrap_or_else(|| "20".to_string()),
                build_command.clone().unwrap_or_else(|| "npm run build".to_string()),
                output_directory.clone().unwrap_or_else(|| "dist".to_string()),
            ),
            _ => ("htmx".to_string(), "20".to_string(), "npm run build".to_string(), "dist".to_string()),
        };

        format!(r#"name: Deploy {framework} App

on:
  push:
    branches: [ main, {env_name} ]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '{node_version}'

      - name: Install dependencies
        run: npm ci

      - name: Build {framework} app
        run: {build_command}
        env:
          NODE_ENV: production

      - name: Package artifacts
        run: tar -czf /tmp/artifact.tar.gz -C ./{output_dir} .

      - name: Deploy via Gateway
        run: |
          curl -X POST ${{{{ DEPLOY_GATEWAY_URL }}}}/deploy \
            -H "X-Deploy-Key: ${{{{ DEPLOY_KEY }}}}" \
            -H "Content-Type: application/json" \
            -d '{{
              "app_name": "${{{{ gitea.repository_name }}}}",
              "org": "${{{{ gitea.repository_owner }}}}",
              "project_type": "app",
              "artifact_url": "file:///tmp/artifact.tar.gz",
              "environment": "{env_name}"
            }}'
        env:
          DEPLOY_GATEWAY_URL: ${{{{ secrets.DEPLOY_GATEWAY_URL }}}}
          DEPLOY_KEY: ${{{{ secrets.DEPLOY_KEY }}}}
"#)
    }

    pub(super) fn generate_site_workflow(&self, environment: &DeploymentEnvironment) -> String {
        let env_name = environment.to_string();

        format!(r#"name: Deploy Static Site

on:
  push:
    branches: [ main, {env_name} ]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: npm ci

      - name: Build site
        run: npm run build
        env:
          NODE_ENV: production

      - name: Package artifacts
        run: tar -czf /tmp/artifact.tar.gz -C ./dist .

      - name: Deploy via Gateway
        run: |
          curl -X POST ${{{{ DEPLOY_GATEWAY_URL }}}}/deploy \
            -H "X-Deploy-Key: ${{{{ DEPLOY_KEY }}}}" \
            -H "Content-Type: application/json" \
            -d '{{
              "app_name": "${{{{ gitea.repository_name }}}}",
              "org": "${{{{ gitea.repository_owner }}}}",
              "project_type": "site",
              "artifact_url": "file:///tmp/artifact.tar.gz",
              "environment": "{env_name}"
            }}'
        env:
          DEPLOY_GATEWAY_URL: ${{{{ secrets.DEPLOY_GATEWAY_URL }}}}
          DEPLOY_KEY: ${{{{ secrets.DEPLOY_KEY }}}}
"#)
    }
}
