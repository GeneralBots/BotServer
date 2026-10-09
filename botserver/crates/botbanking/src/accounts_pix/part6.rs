use super::*;

pub(super) fn default_settings() -> BankingSettings {
    BankingSettings {
        tolerance_cents: 1,
        date_window_days: 3,
        auto_approve_under: "500".to_string(),
        notify_on_unmatched: true,
        webhook: String::new(),
    }
}
pub async fn get_settings(headers: HeaderMap) -> Result<Json<BankingSettings>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Integer)] tolerance_cents: i32,
        #[diesel(sql_type = diesel::sql_types::Integer)] date_window_days: i32,
        #[diesel(sql_type = diesel::sql_types::Numeric)] auto_approve_under: Decimal,
        #[diesel(sql_type = diesel::sql_types::Bool)] notify_on_unmatched: bool,
        #[diesel(sql_type = diesel::sql_types::Text)] webhook: String,
    }
    let row: Option<Row> = diesel::sql_query(
        "SELECT tolerance_cents, date_window_days, auto_approve_under, notify_on_unmatched, webhook
         FROM banking_settings WHERE branch_id = $1 LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .optional()
    .map_err(db::map_diesel_err)?;
    match row {
        Some(r) => ok(BankingSettings {
            tolerance_cents: r.tolerance_cents,
            date_window_days: r.date_window_days,
            auto_approve_under: decimal_to_string(r.auto_approve_under),
            notify_on_unmatched: r.notify_on_unmatched,
            webhook: r.webhook,
        }),
        None => ok(default_settings()),
    }
}
pub async fn save_settings(
    headers: HeaderMap,
    Json(req): Json<SaveSettings>,
) -> Result<Json<BankingSettings>, (StatusCode, String)> {
    let mut current = get_settings(headers.clone()).await?.0;
    if let Some(v) = req.tolerance_cents { current.tolerance_cents = v.clamp(0, 100); }
    if let Some(v) = req.date_window_days { current.date_window_days = v.clamp(0, 30); }
    if let Some(v) = req.auto_approve_under {
        if let Ok(d) = v.parse::<Decimal>() { current.auto_approve_under = decimal_to_string(d); }
    }
    if let Some(v) = req.notify_on_unmatched { current.notify_on_unmatched = v; }
    if let Some(v) = req.webhook { current.webhook = v; }

    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let auto = current.auto_approve_under.parse::<Decimal>().unwrap_or_default();
    let now = Utc::now();
    let n = diesel::sql_query(
        "UPDATE banking_settings SET tolerance_cents=$2, date_window_days=$3, auto_approve_under=$4, notify_on_unmatched=$5, webhook=$6, updated_at=$7 WHERE branch_id=$1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Integer, _>(current.tolerance_cents)
    .bind::<diesel::sql_types::Integer, _>(current.date_window_days)
    .bind::<diesel::sql_types::Numeric, _>(auto)
    .bind::<diesel::sql_types::Bool, _>(current.notify_on_unmatched)
    .bind::<diesel::sql_types::Text, _>(&current.webhook)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    if n == 0 {
        diesel::sql_query(
            "INSERT INTO banking_settings (branch_id, tolerance_cents, date_window_days, auto_approve_under, notify_on_unmatched, webhook, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .bind::<diesel::sql_types::Integer, _>(current.tolerance_cents)
        .bind::<diesel::sql_types::Integer, _>(current.date_window_days)
        .bind::<diesel::sql_types::Numeric, _>(auto)
        .bind::<diesel::sql_types::Bool, _>(current.notify_on_unmatched)
        .bind::<diesel::sql_types::Text, _>(&current.webhook)
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .execute(&mut conn)
        .map_err(db::map_diesel_err)?;
    }
    ok(current)
}
pub async fn reset_settings(headers: HeaderMap) -> Result<Json<BankingSettings>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let def = default_settings();
    let auto = def.auto_approve_under.parse::<Decimal>().unwrap_or_default();
    let now = Utc::now();
    diesel::sql_query(
        "DELETE FROM banking_settings WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    diesel::sql_query(
        "INSERT INTO banking_settings (branch_id, tolerance_cents, date_window_days, auto_approve_under, notify_on_unmatched, webhook, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Integer, _>(def.tolerance_cents)
    .bind::<diesel::sql_types::Integer, _>(def.date_window_days)
    .bind::<diesel::sql_types::Numeric, _>(auto)
    .bind::<diesel::sql_types::Bool, _>(def.notify_on_unmatched)
    .bind::<diesel::sql_types::Text, _>(&def.webhook)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    ok(def)
}

// ---------------------------------------------------------------------------
// Stats (header cards)
// ---------------------------------------------------------------------------
