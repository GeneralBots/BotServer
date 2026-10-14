//! API handlers for deployment module
//!
//! Three project types: bots (gbdialog), apps (Incus container), sites (Caddy static).

use axum::{Json, Extension};
use diesel::prelude::*;
use crate::models::Project;
use super::router::DeploymentRouter;
use super::types::*;

pub use botlib::db_pool::DbPool;

pub fn configure_deployment_routes(pool: DbPool) -> axum::Router<()> {
    let pool_ext = Extension(pool);
    axum::Router::new()
        .route("/api/deployment/types", axum::routing::get(get_project_types))
        .route("/api/deployment/deploy", axum::routing::post(deploy_project))
        .route("/api/deployment/stop", axum::routing::post(stop_project))
        .route("/api/deployment/start", axum::routing::post(start_project))
        .route("/api/deployment/status/:org/:app_name", axum::routing::get(get_project_status))
        .route("/api/deployment/projects", axum::routing::get(get_projects))
        // `Router::layer` wraps the routes already registered at call time;
        // routes added afterwards would not receive the pool extension and
        // handlers would fail with "Missing request extension: r2d2::Pool".
        .layer(pool_ext)
}

pub async fn get_project_types() -> Result<Json<ProjectTypesResponse>, DeploymentApiError> {
    let project_types = vec![
        ProjectTypeInfo {
            id: "bot".to_string(),
            name: "Bot".to_string(),
            description: "GB Native bot scripts running in chat mode via .gbdialog".to_string(),
            deploy_target: "none".to_string(),
            features: vec![
                "Runs in BotServer chat mode".to_string(),
                "Stored in MinIO .gbai bucket".to_string(),
                "No ALM repository needed".to_string(),
                "Instant deployment via .bas scripts".to_string(),
            ],
        },
        ProjectTypeInfo {
            id: "app-htmx".to_string(),
            name: "App (HTMX)".to_string(),
            description: "HTMX application deployed to its own Incus container".to_string(),
            deploy_target: "incus-container".to_string(),
            features: vec![
                "Own Incus container (isolated)".to_string(),
                "Full server-side rendering".to_string(),
                "Custom CI/CD via alm-ci".to_string(),
                "Start/stop control".to_string(),
            ],
        },
        ProjectTypeInfo {
            id: "app-react".to_string(),
            name: "App (React)".to_string(),
            description: "React application deployed to its own Incus container".to_string(),
            deploy_target: "incus-container".to_string(),
            features: vec![
                "Own Incus container (isolated)".to_string(),
                "Vite build system".to_string(),
                "Custom CI/CD via alm-ci".to_string(),
                "Start/stop control".to_string(),
            ],
        },
        ProjectTypeInfo {
            id: "app-vue".to_string(),
            name: "App (Vue)".to_string(),
            description: "Vue.js application deployed to its own Incus container".to_string(),
            deploy_target: "incus-container".to_string(),
            features: vec![
                "Own Incus container (isolated)".to_string(),
                "Vite build system".to_string(),
                "Custom CI/CD via alm-ci".to_string(),
                "Start/stop control".to_string(),
            ],
        },
        ProjectTypeInfo {
            id: "site".to_string(),
            name: "Site".to_string(),
            description: "Static HTML/CSS/JS site served by Caddy proxy".to_string(),
            deploy_target: "caddy-static".to_string(),
            features: vec![
                "Zero runtime cost".to_string(),
                "Served by Caddy (proxy container)".to_string(),
                "Custom domain support".to_string(),
                "ALM repository with CI/CD".to_string(),
            ],
        },
    ];

    Ok(Json(ProjectTypesResponse { project_types }))
}

pub async fn deploy_project(
    Extension(pool): Extension<DbPool>,
    Extension(user): Extension<botsecurity_auth::auth_api::types::AuthenticatedUser>,
    Json(request): Json<DeploymentRequest>,
) -> Result<Json<DeploymentResponse>, DeploymentApiError> {
    let effective_user = if !user.user_id.is_nil() {
        Some(user.user_id)
    } else {
        // #1280 — internal callers (vibe agent publish tool) authenticate via
        // X-Internal-Token with a nil session user; the acting user travels in
        // the request body and MUST pass the same deploy-role gate. A request
        // with neither a real session user nor an on-behalf-of user is
        // rejected (no anonymous deploys).
        request.on_behalf_of_user.filter(|id| !id.is_nil())
    };
    let effective_user = match effective_user {
        Some(id) => id,
        None => {
            log::warn!("Deployment denied: no session user and no on_behalf_of_user supplied");
            return Err(DeploymentApiError::Forbidden(
                "forbidden: deploying requires an authenticated user".to_string(),
            ));
        }
    };
    if let Err(msg) = super::rbac_guard::require_deploy_role(&pool, effective_user, request.project_id) {
        log::warn!("Deployment denied for user {effective_user}: {msg}");
        return Err(DeploymentApiError::Forbidden(msg));
    }
    log::info!(
        "Deployment request: org={:?}, app={}, type={}, env={}",
        request.organization,
        request.app_name,
        request.project_type,
        request.environment
    );

    let (project_type, dt_target) = match request.project_type.as_str() {
        "bot" => (ProjectType::Bot, DeployTarget::None),
        "site" => (ProjectType::Site, DeployTarget::CaddyStatic),
        app_type if app_type.starts_with("app-") => {
            let framework = request.framework.clone()
                .unwrap_or_else(|| app_type.strip_prefix("app-").unwrap_or("unknown").to_string());
            let pt = ProjectType::App {
                framework,
                node_version: request.node_version.clone(),
                build_command: request.build_command.clone(),
                output_directory: request.output_directory.clone(),
            };
            let dt = DeployTarget::from(&pt);
            (pt, dt)
        }
        _ => {
            return Err(DeploymentApiError::ValidationError(format!(
                "Unknown project type: {}",
                request.project_type
            )));
        }
    };

    let environment = match request.environment.as_str() {
        "development" => DeploymentEnvironment::Development,
        "staging" => DeploymentEnvironment::Staging,
        "production" => DeploymentEnvironment::Production,
        _ => DeploymentEnvironment::Development,
    };

    let (forgejo_url, alm_token, _org) = botcoresecrets::alm_config();
    let forgejo_token = if alm_token.is_empty() { None } else { Some(alm_token) };

    let organization = request.organization
        .or_else(|| std::env::var("FORGEJO_DEFAULT_ORG").ok())
        .unwrap_or_else(|| "generalbots".to_string());

    let ci_cd_enabled = request.ci_cd_enabled.unwrap_or(true);

    let config = DeploymentConfig {
        organization: organization.clone(),
        app_name: request.app_name.clone(),
        project_type,
        deploy_target: dt_target.clone(),
        environment,
        custom_domain: request.custom_domain.clone(),
        ci_cd_enabled,
        database_url: request.database_url.clone(),
    };

    let router = DeploymentRouter::new(forgejo_url, forgejo_token);

    let generated_app = if request.files.is_empty() {
        // No source files provided (e.g. bot projects deploy in gbdialog
        // mode without an ALM repo, or the caller has nothing to push yet).
        GeneratedApp::new(
            config.app_name.clone(),
            format!("{} project", config.project_type),
        )
    } else {
        GeneratedApp {
            name: config.app_name.clone(),
            description: format!("{} project", config.project_type),
            files: request.files,
        }
    };

    let result = router.deploy(config, generated_app).await
        .map_err(|e| DeploymentApiError::DeploymentFailed(e.to_string()))?;

    log::info!(
        "Deployment successful: url={}, repo={}, status={:?}",
        result.url,
        result.repository,
        result.status
    );

    // Save project metadata to database (Issue #526)
    if let Ok(mut conn) = pool.get() {
        let now = chrono::Utc::now();
        let new_project = Project {
            id: uuid::Uuid::new_v4(),
            org: organization.clone(),
            name: request.app_name.clone(),
            project_type: request.project_type.clone(),
            deploy_target: format!("{:?}", dt_target).to_lowercase(),
            repo_url: Some(result.repository.clone()),
            deploy_url: Some(result.url.clone()),
            container_name: if let DeployTarget::IncusContainer = dt_target {
                Some(format!("{}-{}", organization, request.app_name))
            } else {
                None
            },
            custom_domain: request.custom_domain.clone(),
            environment: request.environment.clone(),
            status: "deployed".to_string(),
            framework: request.framework.clone(),
            description: request.description.clone(),
            created_at: now,
            updated_at: now,
        };

        use crate::schema::projects::dsl::*;
        // Idempotent metadata: re-deploys hit the unique (org, name)
        // constraint, so update the existing row (deploy_url, status, env,
        // container, repo) instead of inserting a duplicate and logging a
        // confusing "duplicate key" error every deploy (#1271 re-deploy).
        let existing = projects
            .filter(org.eq(&organization).and(name.eq(&request.app_name)))
            .first::<Project>(&mut conn)
            .optional();
        match existing {
            Ok(Some(row)) => {
                if let Err(e) = diesel::update(projects.filter(id.eq(row.id)))
                    .set((
                        deploy_url.eq(&result.url),
                        repo_url.eq(&result.repository),
                        container_name.eq(&new_project.container_name),
                        custom_domain.eq(&request.custom_domain.clone()),
                        environment.eq(&request.environment.clone()),
                        status.eq("deployed"),
                        updated_at.eq(now),
                    ))
                    .execute(&mut conn)
                {
                    log::error!("Failed to update project metadata in database: {}", e);
                }
            }
            _ => {
                if let Err(e) = diesel::insert_into(projects)
                    .values(&new_project)
                    .execute(&mut conn)
                {
                    log::error!("Failed to persist project metadata in database: {}", e);
                }
            }
        }
    } else {
        log::error!("Failed to obtain database connection to persist project metadata");
    }

    Ok(Json(DeploymentResponse {
        success: true,
        url: Some(result.url),
        repository: Some(result.repository),
        project_type: Some(result.project_type),
        deploy_target: Some(result.deploy_target),
        status: Some(format!("{:?}", result.status)),
        error: None,
    }))
}

pub async fn stop_project(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<DeployGatewayResponse>, DeploymentApiError> {
    let app_name = body.get("app_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let org = body.get("org")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let (forgejo_url, alm_token, _org) = botcoresecrets::alm_config();
    let router = DeploymentRouter::new(forgejo_url, if alm_token.is_empty() { None } else { Some(alm_token) });

    router.stop(&app_name, &org).await
        .map_err(|e| DeploymentApiError::DeploymentFailed(e.to_string()))
        .map(Json)
}

pub async fn start_project(
    Json(body): Json<serde_json::Value>,
) -> Result<Json<DeployGatewayResponse>, DeploymentApiError> {
    let app_name = body.get("app_name")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let org = body.get("org")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let (forgejo_url, alm_token, _org) = botcoresecrets::alm_config();
    let router = DeploymentRouter::new(forgejo_url, if alm_token.is_empty() { None } else { Some(alm_token) });

    router.start(&app_name, &org).await
        .map_err(|e| DeploymentApiError::DeploymentFailed(e.to_string()))
        .map(Json)
}

pub async fn get_project_status(
    axum::extract::Path((org, app_name)): axum::extract::Path<(String, String)>,
) -> Result<Json<DeployGatewayResponse>, DeploymentApiError> {
    let (forgejo_url, alm_token, _org) = botcoresecrets::alm_config();
    let router = DeploymentRouter::new(forgejo_url, if alm_token.is_empty() { None } else { Some(alm_token) });

    router.status(&app_name, &org).await
        .map_err(|e| DeploymentApiError::DeploymentFailed(e.to_string()))
        .map(Json)
}

pub async fn get_projects(
    Extension(pool): Extension<DbPool>,
) -> Result<Json<Vec<Project>>, DeploymentApiError> {
    let mut conn = pool.get()
        .map_err(|e| DeploymentApiError::DeploymentFailed(format!("Database connection failed: {}", e)))?;

    use crate::schema::projects::dsl::*;
    let list = projects.load::<Project>(&mut conn)
        .map_err(|e| DeploymentApiError::DeploymentFailed(format!("Failed to load projects: {}", e)))?;

    Ok(Json(list))
}
