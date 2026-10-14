//! Deployment router for General Bots platform
//!
//! Three project types: bots (gbdialog, no ALM), apps (ALM → Incus), sites (ALM → Caddy).
//! Organizations map to ALM (Forgejo) organizations. Projects map to ALM repos.

use super::forgejo::ForgejoClient;
use super::gateway::DeployGateway;
use super::types::*;

pub struct DeploymentRouter {
    forgejo_url: String,
    forgejo_token: Option<String>,
    gateway: Option<DeployGateway>,
}

impl DeploymentRouter {
    pub fn new(forgejo_url: String, forgejo_token: Option<String>) -> Self {
        let gateway_config = super::gateway::DeployGatewayConfig::default();
        let gateway = if gateway_config.deploy_key.is_empty() {
            None
        } else {
            Some(DeployGateway::new(gateway_config))
        };
        Self {
            forgejo_url,
            forgejo_token,
            gateway,
        }
    }

    pub fn with_gateway(mut self, gateway: DeployGateway) -> Self {
        self.gateway = Some(gateway);
        self
    }

    pub async fn deploy(
        &self,
        config: DeploymentConfig,
        generated_app: GeneratedApp,
    ) -> Result<DeploymentResult, DeploymentError> {
        log::info!(
            "Deploying {} project: {}/{} to {} environment (target: {})",
            config.project_type,
            config.organization,
            config.app_name,
            config.environment,
            config.deploy_target,
        );

        if matches!(config.deploy_target, DeployTarget::None) {
            return Ok(DeploymentResult {
                url: String::new(),
                repository: String::new(),
                project_type: config.project_type.to_string(),
                deploy_target: config.deploy_target.to_string(),
                environment: config.environment.to_string(),
                status: DeploymentStatus::Deployed,
                metadata: serde_json::json!({
                    "org": config.organization,
                    "app_name": config.app_name,
                    "note": "Bot projects run in gbdialog mode, no ALM repo needed",
                }),
            });
        }

        let token = self.forgejo_token.clone()
            .ok_or_else(|| DeploymentError::ConfigurationError("FORGEJO_TOKEN not configured".to_string()))?;

        let client = ForgejoClient::new(self.forgejo_url.clone(), token);

        let repo = client.create_repository(
            &config.organization,
            &config.app_name,
            &generated_app.description,
            false,
        ).await?;

        // The API returns the public clone URL (e.g.
        // https://alm.pragmatismo.com.br/...), which is not resolvable from
        // inside the bot container. Rewrite it to the configured internal
        // Forgejo base so the push reaches the local ALM.
        let repo_url = rewrite_clone_url(&self.forgejo_url, &repo.clone_url);
        log::info!("Repository created/verified: {}", repo.clone_url);

        let branch = config.environment.to_string();
        client.push_app(&repo_url, &generated_app, &branch).await?;

        if config.ci_cd_enabled {
            client.create_cicd_workflow(
                &repo_url,
                &config.project_type,
                &config.deploy_target,
                &config.environment,
            ).await?;
            log::info!("CI/CD workflow created for {}/{}", config.organization, config.app_name);
        }

        let url = self.build_deployment_url(&config);

        let status = if config.ci_cd_enabled {
            DeploymentStatus::Building
        } else if let Some(ref gateway) = self.gateway {
            let gateway_request = DeployGatewayRequest {
                app_name: config.app_name.clone(),
                org: config.organization.clone(),
                project_type: config.project_type.clone(),
                artifact_url: String::new(),
                environment: config.environment.clone(),
                database_url: config.database_url.clone(),
            };
            match gateway.deploy(gateway_request).await {
                Ok(resp) if resp.success => DeploymentStatus::Deployed,
                Ok(resp) => {
                    log::warn!("Gateway deploy returned error: {:?}", resp.error);
                    DeploymentStatus::Failed
                }
                Err(e) => {
                    log::warn!("Gateway deploy failed: {e}");
                    DeploymentStatus::Building
                }
            }
        } else {
            DeploymentStatus::Building
        };

        Ok(DeploymentResult {
            url,
            repository: repo.clone_url,
            project_type: config.project_type.to_string(),
            deploy_target: config.deploy_target.to_string(),
            environment: config.environment.to_string(),
            status,
            metadata: serde_json::json!({
                "org": config.organization,
                "app_name": config.app_name,
                "repo_id": repo.id,
                "forgejo_url": self.forgejo_url,
                "custom_domain": config.custom_domain,
                "deploy_target": config.deploy_target.to_string(),
            }),
        })
    }

    pub async fn stop(&self, app_name: &str, org: &str) -> Result<DeployGatewayResponse, DeploymentError> {
        let gateway = self.gateway.as_ref()
            .ok_or_else(|| DeploymentError::ConfigurationError("Deploy Gateway not configured".to_string()))?;
        gateway.stop_container(app_name, org).await
    }

    pub async fn start(&self, app_name: &str, org: &str) -> Result<DeployGatewayResponse, DeploymentError> {
        let gateway = self.gateway.as_ref()
            .ok_or_else(|| DeploymentError::ConfigurationError("Deploy Gateway not configured".to_string()))?;
        gateway.start_container(app_name, org).await
    }

    pub async fn status(&self, app_name: &str, org: &str) -> Result<DeployGatewayResponse, DeploymentError> {
        let gateway = self.gateway.as_ref()
            .ok_or_else(|| DeploymentError::ConfigurationError("Deploy Gateway not configured".to_string()))?;
        gateway.get_status(app_name, org).await
    }

    fn build_deployment_url(&self, config: &DeploymentConfig) -> String {
        if let Some(ref domain) = config.custom_domain {
            format!("https://{domain}/")
        } else {
            // #1261 — deployed apps are reachable at `{appname}.{domain}`,
            // honoring GB_PLATFORM_DOMAIN (e.g. generalbots.org) so the same
            // wildcard zone serves bots and published vibe apps.
            let domain = super::gateway_server::published_domain();
            match config.environment {
                DeploymentEnvironment::Production => {
                    format!("https://{}.{domain}/", config.app_name)
                }
                DeploymentEnvironment::Staging => {
                    format!("https://{}-staging.{domain}/", config.app_name)
                }
                DeploymentEnvironment::Development => {
                    format!("https://{}-dev.{domain}/", config.app_name)
                }
            }
        }
    }
}

/// Rewrite the Forgejo API's public clone URL to the configured internal
/// base so pushes reach the local ALM. The API returns e.g.
/// `https://alm.pragmatismo.com.br/org/repo.git` (not resolvable from the
/// bot container), while the configured base is `http://10.0.0.13:4747`.
/// Keeps the path (`/org/repo.git`) from the original URL.
fn rewrite_clone_url(base_url: &str, clone_url: &str) -> String {
    // "https://alm.pragmatismo.com.br/org/repo.git" → "org/repo.git"
    let path = clone_url.split_once("://").map(|x| x.1)                          // "alm.pragmatismo.com.br/org/repo.git"
        .and_then(|rest| {
            rest.split_once('/').map(|x| x.1)   // "org/repo.git"
        })
        .unwrap_or("")
        .trim_start_matches('/');
    format!("{}/{}", base_url.trim_end_matches('/'), path)
}
