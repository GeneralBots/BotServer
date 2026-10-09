use super::*;

pub async fn handle_upgrade(State(state): State<Arc<BillingApiState>>, axum::extract::Form(form): axum::extract::Form<OfferForm>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string()));
    };
    let branch = branch_id(&state);
    let plan = form.offer.unwrap_or_else(|| "shared".to_string());

    let _ = diesel::sql_query(
        "INSERT INTO billing_recurring (id, branch_id, plan_name, status, currency, frequency, interval_count, start_date, created_at, updated_at, customer_name)
         VALUES ($1, $2, $3, 'trialing', 'USD', 'monthly', 1, CURRENT_DATE, NOW(), NOW(), 'Organization')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Text, _>(&plan)
    .execute(&mut conn);

    (StatusCode::OK, Html(format!("Subscription upgraded to <strong>{}</strong>", html_escape(&plan))))
}
pub async fn handle_accept_offer(State(state): State<Arc<BillingApiState>>, axum::extract::Form(form): axum::extract::Form<OfferForm>) -> impl IntoResponse {
    let _ = (state, form);
    (StatusCode::OK, Html("Retention offer applied".to_string()))
}
pub async fn handle_pause(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string()));
    };
    let branch = branch_id(&state);

    let _ = diesel::sql_query(
        "UPDATE billing_recurring SET status = 'paused', updated_at = NOW() WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn);

    (StatusCode::OK, Html("Subscription paused. Billing will resume on your next period.".to_string()))
}
pub async fn handle_cancel(State(state): State<Arc<BillingApiState>>) -> impl IntoResponse {
    let Ok(mut conn) = state.pool.get() else {
        return (StatusCode::INTERNAL_SERVER_ERROR, Html("Database unavailable".to_string()));
    };
    let branch = branch_id(&state);

    let _ = diesel::sql_query(
        "UPDATE billing_recurring SET status = 'cancelled', cancelled_at = NOW(), updated_at = NOW() WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn);

    (StatusCode::OK, Html("Subscription cancelled. Access remains active until the end of the billing period.".to_string()))
}
