//! `publish::site_3` — split from `site_2` per #1492 (AGENTS.md 450-line
//! rule). Website/Python publish path (#1288/#1290) and the production
//! always-on helper (#1261/#1271).

use super::*;

/// Inputs for the website publish path. Grouped into a struct so the helper
/// keeps a single parameter.
pub(crate) struct SitePublish<'a> {
    pub(crate) project: &'a Project,
    pub(crate) project_id: Uuid,
    pub(crate) is_python_project: bool,
    pub(crate) env: &'a str,
    pub(crate) site_env: crate::site_env::SiteEnv,
    pub(crate) requested_production: bool,
    /// Whether the deploy pipeline sanctioned this production request.
    pub(crate) approved: bool,
    pub(crate) deploy_rev: Option<String>,
    pub(crate) domain: &'a Option<String>,
    pub(crate) launcher_requested: bool,
    pub(crate) widget_requested: bool,
    pub(crate) registry: &'a ProjectRegistry,
    pub(crate) pool: DbPool,
}

impl SitePublish<'_> {
    /// Stage the workspace into the proxy's websites dir and record the
    /// release. Returns the `published` payload the caller returns.
    pub(crate) async fn run(self) -> Result<Value, String> {
        let SitePublish {
            project,
            project_id,
            is_python_project,
            env,
            site_env,
            requested_production,
            approved,
            deploy_rev,
            domain,
            launcher_requested,
            widget_requested,
            registry,
            pool,
        } = self;
        // Sanctioned production publish: stage the workspace into the test
        // twin first, then promote THAT release to the public slug. Both
        // websites stay in step, the public page is always a payload that
        // has already been served (and verified) as the test site, and the
        // production `.prev-N` ring still holds the previous release.
        // #1505 — a sanctioned production deploy ships the pushed git
        // state: resolve the revision once (explicit arg > HEAD) and carry
        // it through both the test staging and the promote-to-prod swap.
        let deploy_rev = crate::deploy_source::deploy_revision(project, deploy_rev.as_deref())?;
        let (proxy_url, route) = if site_env == crate::site_env::SiteEnv::Production {
            // The test twin's URL is intentionally discarded: the public
            // release is what this call reports, and the twin's own route
            // is refreshed by the staging deploy above.
            crate::proxy_sites::deploy_site_to_proxy_env(
                project,
                is_python_project,
                crate::site_env::SiteEnv::Test,
                &pool,
                deploy_rev.clone(),
            )
            .await?;
            let promoted = crate::proxy_sites::promote_site_test_to_prod(
                project,
                is_python_project,
                &pool,
            )
            .await?;
            (promoted, "promoted-from-test-twin".to_string())
        } else {
            crate::proxy_sites::deploy_site_to_proxy_env(
                project,
                is_python_project,
                site_env,
                &pool,
                deploy_rev.clone(),
            )
            .await?
        };
        // The recorded env is the canonical site env (test/production) so
        // the UI can resolve the right preview URL per environment.
        let site_env_name = site_env.as_str();
        let deployment = serde_json::json!({
            "env": site_env_name,
            "at": chrono::Utc::now().to_rfc3339(),
            "url": proxy_url,
            "container": "proxy",
            "deploy_target": "proxy-websites",
            "caddy_route": route,
            "domain": domain.clone().unwrap_or_default(),
            "track": "ok",
        });
    registry
        .append_deployment(project_id, &deployment)
        .map_err(|e| format!("record deployment: {e}"))?;
    let launch_info = if launcher_requested || widget_requested {
        let launch = serde_json::json!({
            "enabled": true,
            "kind": if widget_requested { "widget" } else { "app" },
            "at": chrono::Utc::now().to_rfc3339(),
            "env": site_env_name,
        });
        registry
            .set_launcher(project_id, &launch)
            .map_err(|e| format!("record launcher flag: {e}"))?;
        Some(launch)
    } else {
        None
    };
    let binding = match &domain {
        Some(d) => {
            let bind_req = BindDomainRequest {
                domain: d.clone(),
                env: site_env_name.to_string(),
                access: None,
                allowed_emails: None,
            };
            match ProjectDomains::new(pool).bind(project_id, &bind_req).await {
                Ok(b) => serde_json::json!({ "bound": true, "id": b.id, "domain": b.domain, "env": b.env, "container": b.container, "verified": b.verified, "tls_status": b.tls_status }),
                Err(e) => serde_json::json!({ "bound": false, "error": e }),
            }
        }
        None => serde_json::json!({ "bound": false, "error": "no domain provided" }),
    };
    // A production request that was not sanctioned by the deploy pipeline
    // is served by the test twin; report where the release actually went
    // so the agent (and the user) never assume the live page changed.
    let note = if requested_production && !approved {
        format!(
            "release published to the test site ({}) — the public site ({}) is only updated by the Deploy action",
            deployment["url"].as_str().unwrap_or_default(),
            format!("https://{}.{}", crate::proxy_sites::site_slug(&project.name), crate::publish::published_domain()),
        )
    } else {
        String::new()
    };
    Ok(serde_json::json!({
        "published": true,
        "project": project.name,
        "env": site_env_name,
        "requested_env": env,
        "note": note,
        "container": "proxy",
        "deploy_target": "proxy-websites",
        "url": proxy_url,
        "domain": domain,
        "domain_bind": binding,
        "deployment": deployment,
        "launcher": launch_info,
        "history_key": "deployments",
    }))
    }
}

/// Start the published app always-on inside the prod container and point a
/// Caddy route at it (#1261/#1271). Failures are logged, never fatal.
#[cfg(not(target_os = "windows"))]
pub(crate) async fn start_prod_app(
    pool: &DbPool,
    project_name: &str,
    container_name: &str,
    files: &[Value],
    deploy_db_url: Option<&str>,
    repo_name: &str,
) {
    // 0 = no host proxy device (see run_dev_app): prod apps are served
    // through the Caddy domain route -> container IP:3000, and the old
    // default of 80 collided with the container's listeners.
    let host_port = std::env::var("VIBE_PROD_APP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(0);
    // Start the app always-on. `run_dev_app` may return Err solely
    // because the host port-80 proxy device cannot bind (Caddy owns :80
    // on the host) — irrelevant for prod, which is served through the
    // domain route; the app service inside the container
    // (`vibe-app.service`, Restart=always) is what matters.
    // #1386 — production VMs get the project's PUBLIC database; the dev
    // twin keeps the `_dev` one (see run_project_app). Reuse the URL
    // already resolved for the deployment body (same env → same DB).
    match VmLifecycle::new(pool.clone()).run_dev_app(
        container_name,
        files,
        host_port,
        deploy_db_url,
    ) {
        Ok(_) => log::info!(
            "Vibe publish {}: started app always-on in prod container {}",
            project_name,
            container_name
        ),
        Err(e) => log::warn!(
            "Vibe publish {}: could not start prod app in {}: {e}",
            project_name,
            container_name
        ),
    }
    // #1261 — the published app is reachable at `{repo}.{domain}` (the
    // same wildcard zone as bots). Register a Caddy reverse-proxy route
    // to the prod container unconditionally: the app listens on :3000
    // inside the container (`run_dev_app` pins PORT=3000) and the proxy
    // container cannot resolve `{container}.incus` names, so dial the
    // container's real IP. Route failure is logged but non-fatal.
    // (#1271 — this same host is the public URL recorded below.)
    let host = format!("{repo_name}.{}", published_domain());
    let dial = match VmLifecycle::new(pool.clone()).linux_ip(container_name) {
        Ok(Some(ip)) => format!("{ip}:3000"),
        Ok(None) => {
            log::warn!(
                "Vibe publish {}: no IPv4 found for {} — published URL {} will not route",
                project_name,
                container_name,
                host
            );
            String::new()
        }
        Err(e) => {
            log::warn!(
                "Vibe publish {}: could not resolve IP for {}: {e}",
                project_name,
                container_name
            );
            String::new()
        }
    };
    if !dial.is_empty() {
        match crate::caddy::upsert_route_to(&host, &dial, "public").await {
            Ok(route) => log::info!(
                "Vibe publish {}: Caddy route {} -> {dial}",
                project_name,
                route.route_id
            ),
            Err(e) => log::warn!(
                "Vibe publish {}: Caddy route for {host} failed: {e}",
                project_name
            ),
        }
    }
}
