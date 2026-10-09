use super::*;

/// Shared business logic for the cash-flow diagnosis (#724). Used by the
/// REST handler and by the LLM api-command catalog so chat can answer
/// financial-health questions over WhatsApp.
pub async fn cashflow_diagnosis_inner(
    pool: &botcore::shared::utils::DbPool,
    bot_id: &Uuid,
    caller_branch: Option<Uuid>,
    period_q: Option<&str>,
) -> Result<DiagnosisResponse, String> {
    let scope = resolve_bot_scope(pool, bot_id, caller_branch)
        .ok_or_else(|| "Bot not found".to_string())?;

    let period = period_q.map(|p| p.to_string()).filter(|p| !p.trim().is_empty());

    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;

    #[derive(diesel::QueryableByName)]
    struct SumRow {
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        n: i64,
        #[diesel(sql_type = diesel::sql_types::Double)]
        revenue: f64,
        #[diesel(sql_type = diesel::sql_types::Double)]
        expenses: f64,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        pending: i64,
    }
    let (sum_sql, sum_bind): (&str, Option<String>) = match &period {
        Some(p) => (
            "SELECT count(*) AS n, \
             COALESCE(SUM(CASE WHEN amount >= 0 THEN amount ELSE 0 END), 0)::float8 AS revenue, \
             COALESCE(SUM(CASE WHEN amount < 0 THEN abs(amount) ELSE 0 END), 0)::float8 AS expenses, \
             COALESCE(SUM(CASE WHEN reconciled = false THEN 1 ELSE 0 END), 0) AS pending \
             FROM bank_transactions \
             WHERE bot_id = $1 AND branch_id = $2 AND to_char(transaction_date, 'YYYY-MM') = $3",
            Some(p.clone()),
        ),
        None => (
            "SELECT count(*) AS n, \
             COALESCE(SUM(CASE WHEN amount >= 0 THEN amount ELSE 0 END), 0)::float8 AS revenue, \
             COALESCE(SUM(CASE WHEN amount < 0 THEN abs(amount) ELSE 0 END), 0)::float8 AS expenses, \
             COALESCE(SUM(CASE WHEN reconciled = false THEN 1 ELSE 0 END), 0) AS pending \
             FROM bank_transactions \
             WHERE bot_id = $1 AND branch_id = $2",
            None,
        ),
    };

    let sums: Option<SumRow> = match &sum_bind {
        Some(p) => diesel::sql_query(sum_sql)
            .bind::<diesel::sql_types::Uuid, _>(scope.id)
            .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
            .bind::<diesel::sql_types::Text, _>(p.as_str())
            .get_result(&mut conn)
            .ok(),
        None => diesel::sql_query(sum_sql)
            .bind::<diesel::sql_types::Uuid, _>(scope.id)
            .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
            .get_result(&mut conn)
            .ok(),
    };
    let sums = sums.ok_or_else(|| "No banking data found".to_string())?;

    let (detail_sql, detail_bind) = match &period {
        Some(p) => (
            "SELECT transaction_date, description, amount::float8 AS amount, category, reconciled, bank \
             FROM bank_transactions \
             WHERE bot_id = $1 AND branch_id = $2 AND to_char(transaction_date, 'YYYY-MM') = $3 \
             ORDER BY transaction_date, created_at LIMIT 1000",
            Some(p.clone()),
        ),
        None => (
            "SELECT transaction_date, description, amount::float8 AS amount, category, reconciled, bank \
             FROM bank_transactions \
             WHERE bot_id = $1 AND branch_id = $2 \
             ORDER BY transaction_date, created_at DESC LIMIT 1000",
            None,
        ),
    };

    #[derive(diesel::QueryableByName)]
    struct DetailRow {
        #[diesel(sql_type = diesel::sql_types::Date)]
        transaction_date: NaiveDate,
        #[diesel(sql_type = diesel::sql_types::Text)]
        description: String,
        #[diesel(sql_type = diesel::sql_types::Double)]
        amount: f64,
        #[diesel(sql_type = diesel::sql_types::Text)]
        category: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        reconciled: bool,
        #[diesel(sql_type = diesel::sql_types::Text)]
        bank: String,
    }
    let detail: Vec<DetailRow> = match &detail_bind {
        Some(p) => {
            let rows = diesel::sql_query(detail_sql)
                .bind::<diesel::sql_types::Uuid, _>(scope.id)
                .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
                .bind::<diesel::sql_types::Text, _>(p.as_str())
                .load::<DetailRow>(&mut conn);
            match rows {
                Ok(rows) => rows,
                Err(e) => {
                    log::error!("banking diagnosis detail query failed: {e}");
                    Vec::new()
                }
            }
        }
        None => {
            let rows = diesel::sql_query(detail_sql)
                .bind::<diesel::sql_types::Uuid, _>(scope.id)
                .bind::<diesel::sql_types::Uuid, _>(scope.branch_id)
                .load::<DetailRow>(&mut conn);
            match rows {
                Ok(rows) => rows,
                Err(e) => {
                    log::error!("banking diagnosis detail query failed: {e}");
                    Vec::new()
                }
            }
        }
    };

    let rows: Vec<DiagnosisRow> = detail
        .iter()
        .map(|r| DiagnosisRow {
            date: r.transaction_date.format("%Y-%m-%d").to_string(),
            description: r.description.clone(),
            amount: r.amount,
            category: r.category.clone(),
            reconciled: r.reconciled,
            bank: r.bank.clone(),
        })
        .collect();

    let loaded_rates = bottax::storage::load_rates_from_billing(&mut conn, &scope.branch_id);
    let rates = loaded_rates.unwrap_or_default();
    let source = if loaded_rates.is_some() {
        "billing_tax_rates"
    } else {
        "default"
    };

    Ok(DiagnosisResponse {
        diagnosed: true,
        period,
        scope: serde_json::json!({
            "bot": scope.id,
            "branch": scope.branch_id,
            "name": scope.name,
        }),
        summary: DiagnosisSummary {
            entries: sums.n as usize,
            revenue: round2(sums.revenue),
            expenses: round2(sums.expenses),
            net: round2(sums.revenue - sums.expenses),
            pending_reconciliation: sums.pending as usize,
        },
        detail: rows,
        tax_rates: serde_json::json!({
            "irpj_pct": rates.irpj_pct.to_string(),
            "csll_pct": rates.csll_pct.to_string(),
            "pis_cofins_pct": rates.pis_cofins_pct.to_string(),
            "iss_pct": rates.iss_pct.to_string(),
        }),
        tax_rates_source: source.to_string(),
        reason: None,
    })
}
pub(super) fn parse_csv_rows(csv: &str) -> Vec<CashflowRow> {
    let mut rows: Vec<CashflowRow> = Vec::new();
    let mut lines = csv.lines();
    let header_line = match lines.next() {
        Some(h) => h,
        None => return rows,
    };
    let headers: Vec<String> = header_line
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_lowercase())
        .collect();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let values: Vec<String> = line
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .collect();
        let get = |h: &str| -> Option<String> {
            match headers.iter().position(|x| x == h) {
                Some(idx) => values.get(idx).cloned(),
                None => None,
            }
        };
        let date = get("date")
            .or_else(|| get("data"))
            .or_else(|| get("vencimento"))
            .unwrap_or_default();
        let description = get("description")
            .or_else(|| get("descricao"))
            .or_else(|| get("historico"))
            .unwrap_or_default();
        let amount = get("amount").or_else(|| get("valor")).unwrap_or_default();
        let kind = get("type").or_else(|| get("tipo")).unwrap_or_default();
        if date.is_empty() || amount.is_empty() {
            continue;
        }
        rows.push(CashflowRow {
            date,
            description: Some(description),
            amount,
            kind: Some(kind),
            category: None,
        });
    }
    rows
}
