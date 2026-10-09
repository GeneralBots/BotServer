use super::*;

pub fn resolve_bot_scope(
    pool: &botcore::shared::utils::DbPool,
    bot_id: &Uuid,
    caller_branch: Option<Uuid>,
) -> Option<BotScopeRow> {
    let mut conn = pool.get().ok()?;
    let scope = if *bot_id == Uuid::nil() {
        diesel::sql_query(
            "SELECT id, branch_id, name FROM bots \
             WHERE is_default_for_branch = true ORDER BY created_at ASC LIMIT 1",
        )
        .get_result(&mut conn)
        .ok()
    } else {
        diesel::sql_query("SELECT id, branch_id, name FROM bots WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(bot_id)
            .get_result(&mut conn)
            .ok()
    };
    // Authorize the resolved scope against the caller's tenant (issue #734):
    // a client-supplied bot_id may only reference a bot that belongs to the
    // caller's branch. Anonymous/internal callers pass None (server-resolved).
    match caller_branch {
        Some(branch) => scope.filter(|s: &BotScopeRow| s.branch_id == branch),
        None => scope,
    }
}
pub(super) fn parse_br_amount(s: &str) -> Option<f64> {
    let cleaned: String = s
        .replace(['R', '$', ' '], "")
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == ',' || *c == '.' || *c == '-')
        .collect();
    let parsed = if cleaned.contains(',') && cleaned.contains('.') {
        let last_comma = cleaned.rfind(',').map(|i| i + 1).unwrap_or(0);
        let last_dot = cleaned.rfind('.').map(|i| i + 1).unwrap_or(0);
        let sep = if last_comma > last_dot { ',' } else { '.' };
        if sep == ',' {
            cleaned.replace('.', "").replace(',', ".")
        } else {
            cleaned.replace(',', "")
        }
    } else if cleaned.contains(',') {
        cleaned.replace(',', ".")
    } else {
        cleaned
    };
    parsed.parse::<f64>().ok()
}
pub(super) fn parse_br_date(s: &str) -> Option<NaiveDate> {
    let trimmed = s.trim();
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Some(d);
    }
    if let Ok(d) = NaiveDate::parse_from_str(trimmed, "%d/%m/%Y") {
        return Some(d);
    }
    NaiveDate::parse_from_str(trimmed, "%Y/%m/%d").ok()
}
pub(super) fn is_expense(kind: &str, amount: f64) -> bool {
    amount < 0.0
        || kind.contains("despesa")
        || kind.contains("saida")
        || kind.contains("debito")
}
pub(super) fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
/// Validates and normalizes one sheet row into a signed bank-transaction row.
pub(super) fn normalize_row(row: &CashflowRow) -> Result<(NaiveDate, String, f64, String, String), serde_json::Value> {
    let date = parse_br_date(&row.date).ok_or_else(|| {
        serde_json::json!({ "row": row, "reason": format!("invalid date '{}'", row.date) })
    })?;
    let raw_amount = parse_br_amount(&row.amount).ok_or_else(|| {
        serde_json::json!({ "row": row, "reason": format!("invalid amount '{}'", row.amount) })
    })?;
    let kind = row
        .kind
        .clone()
        .unwrap_or_default()
        .to_lowercase();
    let negative = is_expense(&kind, raw_amount);
    let amount = if negative { -raw_amount.abs() } else { raw_amount.abs() };
    let category = if negative { "despesa".to_string() } else { "receita".to_string() };
    let description = row
        .description
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok((date, description, amount, category, kind))
}
/// Imports valid cash-flow rows into `bank_transactions` (reconciled=false)
/// so the existing reconciliation pipeline can process them later.
pub async fn import_cashflow(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(req): Json<CashflowImportRequest>,
) -> Result<Json<CashflowImportResponse>, (StatusCode, Json<serde_json::Value>)> {
    let err = |code: StatusCode, msg: &str| (code, Json(serde_json::json!({ "error": msg })));
    let bot_id = req
        .bot_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(Uuid::nil());
    let drive = state.drive.as_ref();
    let caller_branch = botcore::shared::tenant::branch_from_claims(&headers);
    match cashflow_import_inner(
        &state.conn,
        drive,
        &bot_id,
        caller_branch,
        req.file_key.as_deref(),
        req.rows,
        req.period,
    )
    .await
    {
        Ok(resp) => Ok(Json(resp)),
        Err(msg) => Err(err(StatusCode::BAD_REQUEST, &msg)),
    }
}
/// Shared business logic for the cash-flow import (issue #724). Used by the
/// REST handler and by the LLM api-command catalog so chat can adjust a
/// month's entries over any channel.
pub async fn cashflow_import_inner(
    pool: &botcore::shared::utils::DbPool,
    drive: Option<&Arc<dyn botlib::traits::DriveRepository>>,
    bot_id: &Uuid,
    caller_branch: Option<Uuid>,
    file_key: Option<&str>,
    rows: Option<Vec<CashflowRow>>,
    period: Option<String>,
) -> Result<CashflowImportResponse, String> {
    let scope = resolve_bot_scope(pool, bot_id, caller_branch)
        .ok_or_else(|| "Bot not found".to_string())?;

    // Sheet content: explicit drive key or supplied rows.
    let csv_text = if let Some(file_key) = file_key {
        let drive = drive.ok_or_else(|| "Drive unavailable".to_string())?;
        // An org-hosted bot's sheet lives in the org workspace bucket
        // (`{slug}.gborg`), not in a `{bot}.gbai` bucket that nothing reads,
        // so the location is resolved from the bot row rather than the name.
        let location = {
            let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
            botbasic_core::utils::bot_drive_location_for(&mut conn, scope.id)
        };
        let key = location.key_for(file_key);
        let bytes = drive
            .get_object(&location.bucket, &key)
            .await
            .map_err(|e| format!("Sheet not found: {e}"))?;
        Some(String::from_utf8_lossy(&bytes).to_string())
    } else {
        None
    };

    let rows: Vec<CashflowRow> = if let Some(rows) = rows {
        rows
    } else if let Some(text) = csv_text {
        parse_csv_rows(&text)
    } else {
        return Err("Provide 'rows' or 'file_key'".to_string());
    };

    let mut normalized: Vec<(NaiveDate, String, f64, String)> = Vec::new();
    let mut invalid: Vec<serde_json::Value> = Vec::new();
    for row in &rows {
        match normalize_row(row) {
            Ok((date, description, amount, category, _kind)) => {
                normalized.push((date, description, amount, category));
            }
            Err(invalid_row) => invalid.push(invalid_row),
        }
    }

    let period = period
        .filter(|p| !p.trim().is_empty())
        .or_else(|| {
            normalized
                .first()
                .map(|(date, _, _, _)| date.format("%Y-%m").to_string())
        })
        .unwrap_or_else(|| Utc::now().format("%Y-%m").to_string());

    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    // Idempotent per period: replace the previous import of the same marker.
    let marker = format!("{IMPORT_BANK_MARKER}:{period}");
    diesel::sql_query(
        "DELETE FROM bank_transactions WHERE bot_id = $1 AND branch_id = $2 AND bank = $3",
    )
    .bind::<diesel::sql_types::Uuid, _>(scope.id)
    .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
    .bind::<diesel::sql_types::Text, _>(&marker)
    .execute(&mut conn)
    .map_err(|e| format!("Cleanup error: {e}"))?;

    let mut revenue = 0.0f64;
    let mut expenses = 0.0f64;
    for (date, description, amount, category) in &normalized {
        if *amount >= 0.0 {
            revenue += amount;
        } else {
            expenses += amount.abs();
        }
        diesel::sql_query(
            "INSERT INTO bank_transactions \
             (id, bot_id, branch_id, bank, account, transaction_date, description, amount, category, reconciled, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false, NOW())",
        )
        .bind::<diesel::sql_types::Uuid, _>(Uuid::new_v4())
        .bind::<diesel::sql_types::Uuid, _>(scope.id)
        .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
        .bind::<diesel::sql_types::Text, _>(&marker)
        .bind::<diesel::sql_types::Text, _>("cashflow")
        .bind::<diesel::sql_types::Date, _>(*date)
        .bind::<diesel::sql_types::Text, _>(description)
        .bind::<diesel::sql_types::Double, _>(*amount)
        .bind::<diesel::sql_types::Text, _>(category)
        .execute(&mut conn)
        .map_err(|e| format!("Insert error: {e}"))?;
    }

    Ok(CashflowImportResponse {
        imported: normalized.len(),
        skipped_invalid: invalid.len(),
        invalid,
        period,
        revenue: round2(revenue),
        expenses: round2(expenses),
        net: round2(revenue - expenses),
    })
}
/// Returns the cash-flow result for the whole branch (issue #724). The
/// diagnosis runs on the global `bank_transactions` data of the bot's
/// branch — an import changes the cash flow, so it is never scoped to a
/// single import. The response has both a summary and the transaction
/// detail, plus the resolved tax rates so the LLM can reason and experiment.
pub async fn cashflow_diagnosis(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Query(query): Query<DiagnosisQuery>,
) -> Result<Json<DiagnosisResponse>, (StatusCode, Json<serde_json::Value>)> {
    let err = |code: StatusCode, msg: &str| (code, Json(serde_json::json!({ "error": msg })));
    let bot_id = query
        .bot_id
        .as_deref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .unwrap_or(Uuid::nil());
    let caller_branch = botcore::shared::tenant::branch_from_claims(&headers);
    match cashflow_diagnosis_inner(&state.conn, &bot_id, caller_branch, query.period.as_deref()).await {
        Ok(resp) => Ok(Json(resp)),
        Err(msg) => Err(err(StatusCode::BAD_REQUEST, &msg)),
    }
}
