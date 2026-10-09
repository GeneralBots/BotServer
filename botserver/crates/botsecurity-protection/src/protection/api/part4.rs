use super::*;

pub(super) async fn security_fix_handler(
) -> Result<Json<ApiResponse<SecurityFixReport>>, (StatusCode, Json<ApiResponse<()>>)> {
    let report = run_security_fix().await;
    if report.success {
        Ok(Json(ApiResponse::success(report)))
    } else {
        // Still return 200 with partial results so caller can inspect each step
        Ok(Json(ApiResponse::success(report)))
    }
}
pub(super) async fn security_fix_status_handler(
) -> Result<Json<ApiResponse<SecurityFixReport>>, (StatusCode, Json<ApiResponse<()>>)> {
    Ok(Json(ApiResponse::success(run_security_status().await)))
}
