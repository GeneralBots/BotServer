use super::*;

pub(super) fn default_statement_format() -> String { "pdf".to_string() }
pub async fn download_statement(
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<(HeaderMap, String), (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::Text)] period: String,
        #[diesel(sql_type = diesel::sql_types::Text)] account_label: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)] opening: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] closing: Decimal,
        #[diesel(sql_type = diesel::sql_types::Timestamptz)] generated_at: chrono::DateTime<Utc>,
        #[diesel(sql_type = diesel::sql_types::Text)] format: String,
    }
    let row: Option<Row> = diesel::sql_query(
        "SELECT period, account_label, opening, closing, generated_at, format FROM banking_statements WHERE id = $1 AND branch_id = $2 LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .optional()
    .map_err(db::map_diesel_err)?;
    let r = match row {
        Some(r) => r,
        None => return Err((StatusCode::NOT_FOUND, format!("Statement {id} not found"))),
    };
    let body = format!(
        "GENERALBOTS STATEMENT\nAccount: {}\nPeriod: {}\nOpening: {}\nClosing: {}\nGenerated: {}\nFormat: {}\n",
        r.account_label, r.period, r.opening, r.closing, r.generated_at.format("%Y-%m-%d %H:%M:%S"), r.format
    );
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static("text/plain; charset=utf-8"),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_str(&format!("attachment; filename=\"statement-{}.txt\"", id))
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("attachment")),
    );
    Ok((headers, body))
}
pub async fn list_statements(headers: HeaderMap) -> Result<Json<Vec<Statement>>, (StatusCode, String)> {
    db::pool()?.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}"))).and_then(|mut conn| {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)] id: Uuid,
            #[diesel(sql_type = diesel::sql_types::Text)] period: String,
            #[diesel(sql_type = diesel::sql_types::Text)] account_label: String,
            #[diesel(sql_type = diesel::sql_types::Numeric)] opening: Decimal,
            #[diesel(sql_type = diesel::sql_types::Numeric)] closing: Decimal,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)] generated_at: chrono::DateTime<Utc>,
            #[diesel(sql_type = diesel::sql_types::Text)] format: String,
        }
        let rows: Vec<Row> = diesel::sql_query(
            "SELECT id, period, account_label, opening, closing, generated_at, format
             FROM banking_statements WHERE branch_id = $1 ORDER BY generated_at DESC LIMIT 500",
        )
        .bind::<diesel::sql_types::Uuid, _>(resolve_branch(&headers))
        .load(&mut conn)
        .map_err(db::map_diesel_err)?;
        ok(rows.into_iter().map(|r| Statement {
            id: r.id, period: r.period, account_label: r.account_label,
            opening: decimal_to_string(r.opening), closing: decimal_to_string(r.closing),
            generated_at: r.generated_at, format: r.format,
        }).collect())
    })
}
pub async fn create_statement(
    headers: HeaderMap,
    Json(req): Json<NewStatement>,
) -> Result<Json<Statement>, (StatusCode, String)> {
    if req.period.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "period is required".to_string()));
    }
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    // Resolve account label and current balance for opening/closing snapshot.
    #[derive(diesel::QueryableByName)]
    struct Acct {
        #[diesel(sql_type = diesel::sql_types::Text)] bank: String,
        #[diesel(sql_type = diesel::sql_types::Text)] account_number: String,
        #[diesel(sql_type = diesel::sql_types::Numeric)] balance: Decimal,
    }
    let acct: Option<Acct> = diesel::sql_query(
        "SELECT bank, account_number, balance FROM banking_accounts WHERE id = $1 AND branch_id = $2 LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(req.account_id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .optional()
    .map_err(db::map_diesel_err)?;
    let (label, balance) = match acct {
        Some(a) => (format!("{} {}", a.bank, a.account_number), a.balance),
        None => (req.period.clone(), Decimal::ZERO),
    };
    let id = Uuid::new_v4();
    let now = Utc::now();
    diesel::sql_query(
        "INSERT INTO banking_statements (id, branch_id, account_label, period, opening, closing, generated_at, format)
         VALUES ($1, $2, $3, $4, $5, $5, $6, $7)",
    )
    .bind::<diesel::sql_types::Uuid, _>(id)
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .bind::<diesel::sql_types::Text, _>(&label)
    .bind::<diesel::sql_types::Text, _>(&req.period)
    .bind::<diesel::sql_types::Numeric, _>(balance)
    .bind::<diesel::sql_types::Timestamptz, _>(now)
    .bind::<diesel::sql_types::Text, _>(&req.format)
    .execute(&mut conn)
    .map_err(db::map_diesel_err)?;
    ok(Statement {
        id, period: req.period, account_label: label,
        opening: decimal_to_string(balance), closing: decimal_to_string(balance),
        generated_at: now, format: req.format,
    })
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------
