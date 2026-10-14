use super::*;

/// Generic over the router state so the crate keeps no dependency on the
/// application core: the caller owns the `AppState` it attaches (#1501).
pub fn configure_protection_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/api/security/protection/status", get(get_all_status))
        .route("/api/security/fix", post(security_fix_handler))
        .route("/api/security/fix/status", get(security_fix_status_handler))
        .route(
            "/api/security/protection/:tool/status",
            get(get_tool_status),
        )
        .route("/api/security/protection/:tool/install", post(install_tool))
        .route(
            "/api/security/protection/:tool/uninstall",
            post(uninstall_tool),
        )
        .route("/api/security/protection/:tool/start", post(start_service))
        .route("/api/security/protection/:tool/stop", post(stop_service))
        .route(
            "/api/security/protection/:tool/enable",
            post(enable_service),
        )
        .route(
            "/api/security/protection/:tool/disable",
            post(disable_service),
        )
        .route("/api/security/protection/:tool/run", post(run_scan))
        .route("/api/security/protection/:tool/report", get(get_report))
        .route(
            "/api/security/protection/:tool/update",
            post(update_definitions),
        )
        .route("/api/security/protection/:tool/auto", post(toggle_auto))
        .route(
            "/api/security/protection/clamav/quarantine",
            get(get_quarantine),
        )
        .route(
            "/api/security/protection/clamav/quarantine/:id",
            post(remove_from_quarantine),
        )
}
pub(super) fn parse_tool(tool_name: &str) -> Result<ProtectionTool, (StatusCode, Json<ApiResponse<()>>)> {
    ProtectionTool::from_str(tool_name).map_err(|_| {
        (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::error(format!("Unknown tool: {tool_name}"))),
        )
    })
}
pub(super) async fn get_all_status(
) -> Result<Json<ApiResponse<AllStatusResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let manager = get_manager().read().await;
    let status_map = manager.get_all_status().await;
    let tools: Vec<ToolStatus> = status_map.into_values().collect();

    Ok(Json(ApiResponse::success(AllStatusResponse { tools })))
}
pub(super) async fn get_tool_status(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ToolStatus>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.check_tool_status(tool).await {
        Ok(status) => Ok(Json(ApiResponse::success(status))),
        Err(e) => {
            warn!(error = %e, "Failed to get tool status");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to get tool status")),
            ))
        }
    }
}
pub(super) async fn install_tool(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.install_tool(tool).await {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("{tool} installed successfully"),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to install tool");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to install tool")),
            ))
        }
    }
}
pub(super) async fn uninstall_tool(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;

    Err((
        StatusCode::NOT_IMPLEMENTED,
        Json(ApiResponse::error(format!(
            "Uninstall not yet implemented for {tool}"
        ))),
    ))
}
pub(super) async fn start_service(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.start_service(tool).await {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("{tool} service started"),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to start service");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to start service")),
            ))
        }
    }
}
pub(super) async fn stop_service(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.stop_service(tool).await {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("{tool} service stopped"),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to stop service");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to stop service")),
            ))
        }
    }
}
pub(super) async fn enable_service(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.enable_service(tool).await {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("{tool} service enabled"),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to enable service");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to enable service")),
            ))
        }
    }
}
pub(super) async fn disable_service(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.disable_service(tool).await {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("{tool} service disabled"),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to disable service");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to disable service")),
            ))
        }
    }
}
pub(super) async fn run_scan(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ScanResult>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.run_scan(tool).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            warn!(error = %e, "Failed to run scan");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to run scan")),
            ))
        }
    }
}
pub(super) async fn get_report(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.get_report(tool).await {
        Ok(report) => Ok(Json(ApiResponse::success(report))),
        Err(e) => {
            warn!(error = %e, "Failed to get report");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to get report")),
            ))
        }
    }
}
pub(super) async fn update_definitions(
    Path(tool_name): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().read().await;

    match manager.update_definitions(tool).await {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("{tool} definitions updated"),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to update definitions");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to update definitions")),
            ))
        }
    }
}
pub(super) async fn toggle_auto(
    Path(tool_name): Path<String>,
    Json(request): Json<AutoToggleRequest>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    let tool = parse_tool(&tool_name)?;
    let manager = get_manager().write().await;

    let setting = request.setting.as_deref().unwrap_or("update");

    let result = match setting {
        "update" => manager.set_auto_update(tool, request.enabled).await,
        "remediate" => manager.set_auto_remediate(tool, request.enabled).await,
        _ => manager.set_auto_update(tool, request.enabled).await,
    };

    match result {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("{tool} {setting} set to {}", request.enabled),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to toggle auto setting");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to toggle auto setting")),
            ))
        }
    }
}
pub(super) async fn get_quarantine(
) -> Result<Json<ApiResponse<Vec<super::lmd::QuarantinedFile>>>, (StatusCode, Json<ApiResponse<()>>)>
{
    match super::lmd::list_quarantined().await {
        Ok(files) => Ok(Json(ApiResponse::success(files))),
        Err(e) => {
            warn!(error = %e, "Failed to get quarantine list");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to get quarantine list")),
            ))
        }
    }
}
pub(super) async fn remove_from_quarantine(
    Path(file_id): Path<String>,
) -> Result<Json<ApiResponse<ActionResponse>>, (StatusCode, Json<ApiResponse<()>>)> {
    match super::lmd::restore_file(&file_id).await {
        Ok(()) => Ok(Json(ApiResponse::success(ActionResponse {
            success: true,
            message: format!("File {file_id} restored from quarantine"),
        }))),
        Err(e) => {
            warn!(error = %e, "Failed to restore file from quarantine");
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::error("Failed to restore file from quarantine")),
            ))
        }
    }
}
