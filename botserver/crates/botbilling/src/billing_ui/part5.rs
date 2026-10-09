use super::*;

pub(super) async fn handle_subscription_upgrade(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
    Json(req): Json<UpgradeRequest>,
) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return Json(serde_json::json!({
            "success": false,
            "error": "Database unavailable"
        }));
    };

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(&state.pool, &state.get_default_bot));

    match diesel::sql_query(
        "UPDATE billing_recurring \
         SET plan_name = $1, status = 'active', updated_at = NOW() \
         WHERE branch_id = $2",
    )
    .bind::<diesel::sql_types::Text, _>(&req.plan_id)
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .execute(&mut conn)
    {
        Ok(rows) if rows > 0 => Json(serde_json::json!({
            "success": true,
            "plan_id": req.plan_id,
            "message": "Subscription upgraded successfully"
        })),
        Ok(_) => Json(serde_json::json!({
            "success": false,
            "error": "No active subscription found for this branch"
        })),
        Err(e) => {
            tracing::error!("Failed to upgrade subscription: {e}");
            Json(serde_json::json!({
                "success": false,
                "error": "Failed to upgrade subscription"
            }))
        }
    }
}
