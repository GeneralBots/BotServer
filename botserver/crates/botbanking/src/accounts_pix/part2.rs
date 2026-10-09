use super::*;

pub(super) fn default_account_type() -> String { "checking".to_string() }
pub(super) fn default_currency() -> String { "BRL".to_string() }
pub async fn list_accounts(headers: HeaderMap) -> Result<Json<Vec<BankAccount>>, (StatusCode, String)> {
    db::pool()?.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}"))).and_then(|mut conn| {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)] bank: String,
            #[diesel(sql_type = diesel::sql_types::Text)] agency: String,
            #[diesel(sql_type = diesel::sql_types::Text)] account_number: String,
            #[diesel(sql_type = diesel::sql_types::Text)] account_type: String,
            #[diesel(sql_type = diesel::sql_types::Numeric)] balance: Decimal,
            #[diesel(sql_type = diesel::sql_types::Text)] currency: String,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>)] last_sync: Option<chrono::DateTime<Utc>>,
            #[diesel(sql_type = diesel::sql_types::Text)] status: String,
        }
        let rows: Vec<Row> = diesel::sql_query(
            "SELECT id, bank, agency, account_number, account_type, balance, currency, last_sync, status
             FROM banking_accounts WHERE branch_id = $1 ORDER BY bank ASC LIMIT 500",
        )
        .bind::<diesel::sql_types::Uuid, _>(resolve_branch(&headers))
        .load(&mut conn)
        .map_err(db::map_diesel_err)?;
        ok(rows.into_iter().map(|r| BankAccount {
            id: r.id, bank: r.bank, agency: r.agency, account_number: r.account_number,
            account_type: r.account_type, balance: decimal_to_string(r.balance),
            currency: r.currency, last_sync: r.last_sync, status: r.status,
        }).collect())
    })
}
pub async fn create_account(
    headers: HeaderMap,
    Json(req): Json<NewAccount>,
) -> Result<Json<BankAccount>, (StatusCode, String)> {
    if req.bank.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "bank name is required".to_string()));
    }
    let balance = req.balance.parse::<Decimal>().unwrap_or_default();
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    diesel::sql_query(
        "INSERT INTO banking_accounts (id, branch_id, bank, agency, account_number, account_type, balance, currency, last_sync, status)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'active')",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Text, _>(&req.bank)
    .bind::<diesel::sql_types::Text, _>(&req.agency)
    .bind::<diesel::sql_types::Text, _>(&req.account_number)
    .bind::<diesel::sql_types::Text, _>(&req.account_type)
    .bind::<diesel::sql_types::Numeric, _>(balance)
    .bind::<diesel::sql_types::Text, _>(&req.currency)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(Some(now))
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    ok(BankAccount {
        id, bank: req.bank, agency: req.agency, account_number: req.account_number,
        account_type: req.account_type, balance: decimal_to_string(balance),
        currency: req.currency, last_sync: Some(now), status: "active".to_string(),
    })
}
pub async fn sync_account(
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let now = Utc::now();
    let n = diesel::sql_query("UPDATE banking_accounts SET last_sync = $1, status = 'active' WHERE id = $2 AND branch_id = $3")
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .bind::<diesel::sql_types::Uuid, _>(id)
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .execute(&mut conn)
        .map_err(db::map_diesel_err)?;
    if n == 0 { return Err((StatusCode::NOT_FOUND, format!("Account {id} not found"))); }
    ok(serde_json::json!({ "success": true, "account_id": id, "last_sync": now }))
}
pub async fn sync_all_accounts(headers: HeaderMap) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    let now = Utc::now();
    let n = diesel::sql_query("UPDATE banking_accounts SET last_sync = $1, status = 'active' WHERE branch_id = $2")
        .bind::<diesel::sql_types::Timestamptz, _>(now)
        .bind::<diesel::sql_types::Uuid, _>(branch)
        .execute(&mut conn)
        .map_err(db::map_diesel_err)?;
    ok(serde_json::json!({ "success": true, "synced": n, "last_sync": now }))
}

// ---------------------------------------------------------------------------
// PIX transfers
// ---------------------------------------------------------------------------
