use super::*;

impl ForgejoClient {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            base_url,
            token,
            client: Client::new(),
        }
    }

    pub async fn create_repository(
        &self,
        org: &str,
        name: &str,
        description: &str,
        private: bool,
    ) -> Result<ForgejoRepo, ForgejoError> {
        let url = format!("{}/api/v1/org/{org}/repos", self.base_url);

        let payload = CreateRepoRequest {
            name: name.to_string(),
            description: description.to_string(),
            private,
            auto_init: true,
            // Only stock Forgejo gitignore templates are accepted; "React"
            // and "Vite" are not valid templates and make repo creation
            // fail with `GetRepoInitFile[React]: file does not exist`.
            gitignores: Some("Node".to_string()),
            license: Some("MIT".to_string()),
            readme: Some("Default".to_string()),
        };

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ForgejoError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            let repo: ForgejoRepo = response
                .json()
                .await
                .map_err(|e| ForgejoError::JsonError(e.to_string()))?;
            Ok(repo)
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            // Idempotent re-deploys: the repo already exists from a previous
            // deploy. Fetch it instead of failing so the pipeline can push
            // the new commit (the re-deploy must succeed like git does).
            if status == reqwest::StatusCode::CONFLICT && body.contains("already e") {
                let existing = self
                    .client
                    .get(format!("{}/api/v1/repos/{}/{}", self.base_url, org, name))
                    .header("Authorization", format!("token {}", self.token))
                    .send()
                    .await
                    .map_err(|e| ForgejoError::HttpError(e.to_string()))?;
                if existing.status().is_success() {
                    return existing
                        .json()
                        .await
                        .map_err(|e| ForgejoError::JsonError(e.to_string()));
                }
                return Err(ForgejoError::ApiError(format!(
                    "{}: {}",
                    existing.status(),
                    existing.text().await.unwrap_or_default()
                )));
            }
            // Vibe projects derive the ALM org from the branch id
            // (`org=branch`), so the org may not exist yet on a fresh
            // deployment. Auto-create it (admin token) and retry once.
            // Forgejo reports a missing org as 404 in some versions and as
            // 422 "org does not exist" in others — handle both.
            let org_missing = status == reqwest::StatusCode::NOT_FOUND
                || (status == reqwest::StatusCode::UNPROCESSABLE_ENTITY
                    && body.contains("does not exist"));
            if org_missing
                && self.ensure_org(org).await.is_ok() {
                    let retry = self
                        .client
                        .post(&url)
                        .header("Authorization", format!("token {}", self.token))
                        .json(&payload)
                        .send()
                        .await
                        .map_err(|e| ForgejoError::HttpError(e.to_string()))?;
                    if retry.status().is_success() {
                        return retry
                            .json()
                            .await
                            .map_err(|e| ForgejoError::JsonError(e.to_string()));
                    }
                    let retry_status = retry.status();
                    let retry_body = retry.text().await.unwrap_or_default();
                    return Err(ForgejoError::ApiError(format!(
                        "{}: {}",
                        retry_status, retry_body
                    )));
                }
            Err(ForgejoError::ApiError(format!("{}: {}", status, body)))
        }
    }

    /// Create the organization when missing so branch-derived orgs
    /// (org=branch) can own repos without a manual Forgejo setup step.
    async fn ensure_org(&self, org: &str) -> Result<(), ForgejoError> {
        let url = format!("{}/api/v1/orgs", self.base_url);
        let payload = serde_json::json!({
            "username": org,
            "full_name": format!("Workspace {org}"),
            "visibility": "private",
        });
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("token {}", self.token))
            .json(&payload)
            .send()
            .await
            .map_err(|e| ForgejoError::HttpError(e.to_string()))?;
        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ForgejoError::ApiError(format!("{}: {}", status, body)))
        }
    }

    /// Delete a repository (used when a Vibe project is deleted so a
    /// recreated project with the same name starts from a clean repo
    /// instead of inheriting stale history that rejects the seed push).
    pub async fn delete_repository(&self, org: &str, name: &str) -> Result<(), ForgejoError> {
        let url = format!("{}/api/v1/repos/{}/{}", self.base_url, org, name);
        let response = self
            .client
            .delete(&url)
            .header("Authorization", format!("token {}", self.token))
            .send()
            .await
            .map_err(|e| ForgejoError::HttpError(e.to_string()))?;
        // 404 = already gone; treat as success (idempotent delete).
        if response.status().is_success() || response.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(())
        } else {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            Err(ForgejoError::ApiError(format!("{}: {}", status, body)))
        }
    }

    pub async fn push_app(
        &self,
        repo_url: &str,
        app: &GeneratedApp,
        branch: &str,
    ) -> Result<String, DeploymentError> {
        let temp_dir = app.temp_dir()?;
        // The temp dir is a fixed path (`gb-deployments/{name}`) reused
        // across deployments; leaving a previous repo behind makes the next
        // root commit fail with "current tip is not the first parent".
        // Always start from a clean checkout.
        if temp_dir.exists() {
            std::fs::remove_dir_all(&temp_dir)
                .map_err(|e| DeploymentError::GitError(format!("Failed to clean temp dir: {}", e)))?;
        }
        std::fs::create_dir_all(&temp_dir)
            .map_err(|e| DeploymentError::GitError(format!("Failed to create temp dir: {}", e)))?;

        for file in &app.files {
            let file_path = temp_dir.join(&file.path);
            if let Some(parent) = file_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| DeploymentError::GitError(format!("Failed to create parent dir: {}", e)))?;
            }
            std::fs::write(&file_path, &file.content)
                .map_err(|e| DeploymentError::GitError(format!("Failed to write file: {}", e)))?;
        }

        // Empty projects (no source files yet, e.g. created from chat) must
        // still produce a valid initial commit: with an empty tree `git
        // commit` fails with "nothing to commit" and the whole deployment
        // 500s. Seed a placeholder README so the ALM repo gets its initial
        // commit and the pipeline can proceed.
        if app.files.is_empty() {
            std::fs::write(
                temp_dir.join("README.md"),
                format!("# {}\n\nEmpty project scaffolded by General Bots.\n", app.name),
            )
            .map_err(|e| DeploymentError::GitError(format!("Failed to seed README: {}", e)))?;
        }

        let auth_url = self.add_token_to_url(repo_url);
        log::info!("git push url: {auth_url}");
        // Use git CLI entirely (init + add + commit + push) instead of
        // mixing git2 (which has ref-creation and HTTP auth issues) with
        // CLI. The proven flow: init → add → commit → push.
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

        git(&["init", "-q"])?;
        git(&["add", "-A"])?;
        git(&["-c", "user.name=GB Deployer", "-c", "user.email=deployer@generalbots.com",
             "commit", "-m", &format!("Initial commit: {}", app.description)])?;
        let commit_hash = git(&["rev-parse", "HEAD"])?;
        git(&["push", "--force", &auth_url, &format!("HEAD:{branch}")])?;

        Ok(commit_hash)
    }

    pub async fn create_cicd_workflow(
        &self,
        repo_url: &str,
        project_type: &ProjectType,
        deploy_target: &DeployTarget,
        environment: &DeploymentEnvironment,
    ) -> Result<(), DeploymentError> {
        let workflow = match deploy_target {
            DeployTarget::None => return Ok(()),
            DeployTarget::IncusContainer => self.generate_app_workflow(project_type, deploy_target, environment),
            DeployTarget::CaddyStatic => self.generate_site_workflow(environment),
        };

        let workflow_file = GeneratedFile {
            path: ".forgejo/workflows/deploy.yml".to_string(),
            content: workflow.into_bytes(),
        };

        let workflow_app = GeneratedApp {
            name: "workflow".to_string(),
            description: "CI/CD workflow".to_string(),
            files: vec![workflow_file],
        };

        // The workflow file must land on `main` (Forgejo Actions runs
        // workflows from the default branch), but it must never replace the
        // project's history: git-mode workspaces keep `main` as the source
        // of truth, so a force-push of a fresh root commit would destroy it
        // and make the next deploy's push fail with "fetch first".
        // Clone → commit on top → non-force push instead.
        self.push_workflow_preserving(repo_url, &workflow_app).await?;

        Ok(())
    }
}
