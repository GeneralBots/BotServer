use super::*;

pub(super) async fn deploy_to_caddy(
    state: &GatewayState,
    _container_name: &str,
    request: &DeployGatewayRequest,
) -> Result<String, String> {
    // Path-injection guard: `org` and `app_name` come from the deploy request
    // body and become a filesystem path (and Caddy/hostname labels). Only
    // alphanumeric/dash/underscore components are accepted — no separators,
    // no `..`, no traversal out of the sites root.
    let sanitize_component = |v: &str| -> Result<String, String> {
        let cleaned: String = v
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if cleaned.is_empty() {
            return Err(format!("invalid deploy identifier: '{v}'"));
        }
        Ok(cleaned)
    };
    let safe_org = sanitize_component(&request.org)?;
    let safe_app = sanitize_component(&request.app_name)?;
    let site_dir = format!("{}/{}/{}", state.sites_root, safe_org, safe_app);
    tokio::fs::create_dir_all(&site_dir).await
        .map_err(|e| format!("Failed to create site dir: {e}"))?;
    // Containment barrier: after creation, canonicalize and prove the site
    // directory still resolves inside the configured sites root.
    let canon_root = std::path::Path::new(&state.sites_root).canonicalize()
        .map_err(|e| format!("sites root unavailable: {e}"))?;
    let canon_site = std::path::Path::new(&site_dir).canonicalize()
        .map_err(|e| format!("site dir unavailable: {e}"))?;
    if !canon_site.starts_with(&canon_root) {
        return Err(format!("site dir escapes sites root: {}", canon_site.display()));
    }
    let site_dir = canon_site.display().to_string();

    if !request.artifact_url.is_empty() && request.artifact_url.starts_with("file://") {
        let artifact_path = request.artifact_url.strip_prefix("file://").unwrap_or_default();
        // The artifact path is likewise request-controlled: refuse traversal
        // before handing it to `tar -C`.
        if artifact_path.contains("..") || artifact_path.contains('\0') {
            return Err(format!("invalid artifact path: {artifact_path}"));
        }
        // Containment barrier: canonicalize the artifact and require it to
        // exist as a regular file before extraction.
        let canon_artifact = std::path::Path::new(artifact_path).canonicalize()
            .map_err(|e| format!("artifact path unavailable: {e}"))?;
        if !canon_artifact.is_file() {
            return Err(format!("artifact is not a regular file: {}", canon_artifact.display()));
        }
        let artifact_path = canon_artifact.display().to_string();
        let output = tokio::process::Command::new("tar")
            .args(["-xzf", &artifact_path, "-C", &site_dir])
            .output().await
            .map_err(|e| format!("Failed to extract artifact: {e}"))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("Artifact extraction warnings: {stderr}");
        }
    }

    let domain = published_domain();
    let env_suffix = match request.environment {
        DeploymentEnvironment::Production => String::new(),
        DeploymentEnvironment::Staging => "-staging".to_string(),
        DeploymentEnvironment::Development => "-dev".to_string(),
    };
    let app_name = request.app_name.clone();

    let caddy_route = serde_json::json!({
        "@id": format!("{app_name}{env_suffix}"),
        "handle": [{
            "handler": "file_server",
            "root": site_dir,
        }],
        "match": [{ "host": [format!("{app_name}{env_suffix}.{domain}")] }],
    });

    // SSRF guard: the Caddy admin API URL comes from configuration; require an
    // absolute http(s) URL with an explicit host before posting to it.
    let caddy_ok = url::Url::parse(&state.caddy_api_url).is_ok_and(|parsed| {
        matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some()
    });
    let published_url = if caddy_ok {
        let client = reqwest::Client::new();
        let _ = client
            .post(format!("{}/config/apps/http/servers/srv0/routes", state.caddy_api_url))
            .json(&caddy_route)
            .send().await;
        format!("https://{app_name}{env_suffix}.{domain}/")
    } else {
        log::warn!("Deployment skipped: invalid Caddy API URL configuration");
        String::new()
    };

    Ok(published_url)
}
pub(super) async fn run_incus_command(state: &GatewayState, args: &[&str]) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new("incus");
    cmd.args(args);
    cmd.env("INCUS_SOCKET", &state.incus_socket);

    let output = cmd.output().await
        .map_err(|e| format!("Failed to execute incus: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("incus {:?}: {stderr}", args))
    }
}
