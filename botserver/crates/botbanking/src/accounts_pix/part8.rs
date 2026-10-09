use super::*;

pub async fn generate_report(
    headers: HeaderMap,
    Json(req): Json<GenerateReport>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let id = Uuid::new_v4();
    let period = Utc::now().format("%Y-%m").to_string();
    let name = match req.kind.as_str() {
        "cashflow" => "Cash Flow",
        "pnl" => "P&L Summary",
        "tax" => "Tax Obligations",
        _ => "Report",
    }.to_string();
    diesel::sql_query(
        "INSERT INTO banking_reports (id, branch_id, name, kind, period, url, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Text, _>(&name)
    .bind::<diesel::sql_types::Text, _>(&req.kind)
    .bind::<diesel::sql_types::Text, _>(&period)
    .bind::<diesel::sql_types::Text, _>(format!("/api/banking/reports/{id}"))
    .bind::<diesel::sql_types::Timestamptz, _>(Utc::now())
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    ok(serde_json::json!({ "success": true, "id": id, "name": name, "kind": req.kind, "period": period }))
}
