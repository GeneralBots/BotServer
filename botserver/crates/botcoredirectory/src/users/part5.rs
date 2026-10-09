use super::*;

pub async fn update_notification_preferences(
    State(_state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    Json(req): Json<UpdateNotificationPreferencesRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    info!("Updating notification preferences for user: {}: {:?}", user_id, req);

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": user_id,
        "preferences": {
            "email": req.email.unwrap_or(true),
            "push": req.push.unwrap_or(true),
            "sms": req.sms.unwrap_or(false)
        }
    })))
}
