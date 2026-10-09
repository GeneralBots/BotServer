use super::*;

pub(super) fn gen_end_to_end() -> String {
    format!("E{:.0}{}", Utc::now().timestamp_millis() as f64, Uuid::new_v4().simple())
}
pub async fn list_pix(headers: HeaderMap) -> Result<Json<Vec<PixTransfer>>, (StatusCode, String)> {
    db::pool()?.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}"))).and_then(|mut conn| {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)] direction: String,
            #[diesel(sql_type = diesel::sql_types::Text)] key_type: String,
            #[diesel(sql_type = diesel::sql_types::Text)] key_value: String,
            #[diesel(sql_type = diesel::sql_types::Text)] counterparty: String,
            #[diesel(sql_type = diesel::sql_types::Numeric)] amount: Decimal,
            #[diesel(sql_type = diesel::sql_types::Text)] description: String,
            #[diesel(sql_type = diesel::sql_types::Text)] status: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] end_to_end_id: Option<String>,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
        }
        let rows: Vec<Row> = diesel::sql_query(
            "SELECT id, direction, key_type, key_value, counterparty, amount, description, status, end_to_end_id, created_at
             FROM banking_pix_transfers WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 500",
        )
        .bind::<diesel::sql_types::Uuid, _>(resolve_branch(&headers))
        .load(&mut conn)
        .map_err(db::map_diesel_err)?;
        ok(rows.into_iter().map(|r| PixTransfer {
            id: r.id, direction: r.direction, key_type: r.key_type, key_value: r.key_value,
            counterparty: r.counterparty, amount: decimal_to_string(r.amount), description: r.description,
            status: r.status, end_to_end_id: r.end_to_end_id, created_at: r.created_at,
        }).collect())
    })
}
pub async fn pix_transfer(
    headers: HeaderMap,
    Json(req): Json<NewPixTransfer>,
) -> Result<Json<PixTransfer>, (StatusCode, String)> {
    if req.key_value.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "recipient key is required".to_string()));
    }
    let amount = req.amount.parse::<Decimal>().map_err(|_| (StatusCode::BAD_REQUEST, "invalid amount".to_string()))?;
    if amount <= Decimal::ZERO {
        return Err((StatusCode::BAD_REQUEST, "amount must be positive".to_string()));
    }
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    let e2e = gen_end_to_end();
    let _ = req.scheduled_at;
    diesel::sql_query(
        "INSERT INTO banking_pix_transfers (id, branch_id, direction, key_type, key_value, counterparty, amount, description, status, end_to_end_id, created_at)
         VALUES ($1, $2, 'out', $3, $4, $5, $6, $7, 'completed', $8, $9)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Text, _>(&req.key_type)
    .bind::<diesel::sql_types::Text, _>(&req.key_value)
    .bind::<diesel::sql_types::Text, _>(&req.counterparty)
    .bind::<diesel::sql_types::Numeric, _>(amount)
    .bind::<diesel::sql_types::Text, _>(&req.description)
    .bind::<diesel::sql_types::Text, _>(&e2e)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    ok(PixTransfer {
        id, direction: "out".to_string(), key_type: req.key_type, key_value: req.key_value,
        counterparty: req.counterparty, amount: decimal_to_string(amount), description: req.description,
        status: "completed".to_string(), end_to_end_id: Some(e2e), created_at: now,
    })
}
