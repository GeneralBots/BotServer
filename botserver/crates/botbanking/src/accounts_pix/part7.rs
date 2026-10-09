use super::*;

pub async fn stats(headers: HeaderMap) -> Result<Json<BankingStats>, (StatusCode, String)> {
    let branch = resolve_branch(&headers);
    let pool = db::pool()?;
    let mut conn = pool.get().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Pool error: {e}")))?;
    #[derive(diesel::QueryableByName)]
    struct Row {
        #[diesel(sql_type = diesel::sql_types::BigInt)] total: i64,
        #[diesel(sql_type = diesel::sql_types::BigInt)] reconciled: i64,
        #[diesel(sql_type = diesel::sql_types::BigInt)] pending: i64,
        #[diesel(sql_type = diesel::sql_types::Numeric)] total_in: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] total_out: Decimal,
        #[diesel(sql_type = diesel::sql_types::Numeric)] net: Decimal,
    }
    let row: Row = diesel::sql_query(
        "SELECT COUNT(*) AS total,
                COUNT(*) FILTER (WHERE status = 'reconciled') AS reconciled,
                COUNT(*) FILTER (WHERE status != 'reconciled') AS pending,
                COALESCE(SUM(amount) FILTER (WHERE amount > 0), 0) AS total_in,
                COALESCE(ABS(SUM(amount) FILTER (WHERE amount < 0)), 0) AS total_out,
                COALESCE(SUM(amount), 0) AS net
         FROM banking_transactions WHERE branch_id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch)
    .get_result(&mut conn)
    .map_err(db::map_diesel_err)?;
    let match_rate = if row.total > 0 { (row.reconciled as f64 / row.total as f64) * 100.0 } else { 0.0 };
    ok(BankingStats {
        total_in: total_in_f64(row.total_in),
        total_out: total_out_f64(row.total_out),
        net: net_f64(row.net),
        pending: row.pending,
        match_rate,
    })
}
pub(super) fn total_in_f64(d: Decimal) -> f64 { d.to_string().parse::<f64>().unwrap_or(0.0) }
pub(super) fn total_out_f64(d: Decimal) -> f64 { d.to_string().parse::<f64>().unwrap_or(0.0) }
pub(super) fn net_f64(d: Decimal) -> f64 { d.to_string().parse::<f64>().unwrap_or(0.0) }

// ---------------------------------------------------------------------------
// Reports generation
// ---------------------------------------------------------------------------
