use super::*;

pub async fn pix_receive(
    headers: HeaderMap,
    Json(req): Json<NewPixReceive>,
) -> Result<Json<PixTransfer>, (StatusCode, String)> {
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
    diesel::sql_query(
        "INSERT INTO banking_pix_transfers (id, branch_id, direction, key_type, key_value, counterparty, amount, description, status, end_to_end_id, created_at)
         VALUES ($1, $2, 'in', $3, $4, $5, $6, $7, 'completed', $8, $9)",
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
        id, direction: "in".to_string(), key_type: req.key_type, key_value: req.key_value,
        counterparty: req.counterparty, amount: decimal_to_string(amount), description: req.description,
        status: "completed".to_string(), end_to_end_id: Some(e2e), created_at: now,
    })
}
pub async fn export_pix_history(headers: HeaderMap) -> Result<(HeaderMap, String), (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] created_at: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Text)] direction: String,
        #[diesel(sql_type = diesel::sql_types::Text)] key_type: String,
        #[diesel(sql_type = diesel::sql_types::Text)] key_value: String,
        #[diesel(sql_type = diesel::sql_types::Text)] counterparty: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)] amount: Decimal,
        #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)] end_to_end_id: Option<String>,
    }
    let rows: Vec<Row> = diesel::sql_query(
        "SELECT created_at, direction, key_type, key_value, counterparty, amount, status, end_to_end_id
         FROM banking_pix_transfers WHERE branch_id = $1 ORDER BY created_at DESC LIMIT 5000",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .load(&mut conn)
    .map_err(db::map_diesel_err)?;

    let mut csv = String::from("date,direction,key_type,key_value,counterparty,amount,status,end_to_end_id\n");
    for r in rows {
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            r.created_at.format("%Y-%m-%dT%H:%M:%S"),
            r.direction, r.key_type, r.key_value, r.counterparty,
            r.amount.to_string(), r.status, r.end_to_end_id.unwrap_or_default(),
        ));
    }
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/csv; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!(
            "attachment; filename=\"pix-history-{}.csv\"",
            Utc::now().format("%Y%m%d")
        ))
        .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
    );
    Ok((headers, csv))
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------
