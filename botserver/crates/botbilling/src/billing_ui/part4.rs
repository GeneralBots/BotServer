use super::*;

pub(super) async fn handle_invoices_export(
    State(state): State<Arc<BillingApiState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot;

    let rows = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
            .unwrap_or_else(|| crate::get_bot_context(&pool, &get_default_bot));
        #[derive(diesel::QueryableByName)]
        struct CsvRow {
            #[diesel(sql_type = diesel::sql_types::Varchar)]
            invoice_number: String,
            #[diesel(sql_type = diesel::sql_types::Date)]
            issue_date: chrono::NaiveDate,
            #[diesel(sql_type = diesel::sql_types::Numeric)]
            total: BigDecimal,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
            status: Option<String>,
        }
        diesel::sql_query(
            "SELECT invoice_number, issue_date, total, status FROM billing_invoices \
             WHERE branch_id = $1 ORDER BY issue_date DESC LIMIT 1000",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .load::<CsvRow>(&mut conn)
        .ok()
    })
    .await
    .ok()
    .flatten();

    let mut csv_content = String::from("Invoice ID,Date,Amount,Status\n");
    if let Some(rows) = rows {
        for r in rows {
            csv_content.push_str(&format!(
                "{},{},${:.2},{}\n",
                r.invoice_number,
                r.issue_date,
                bd_to_f64(&r.total),
                r.status.as_deref().unwrap_or("draft")
            ));
        }
    }
    (
        StatusCode::OK,
        [
            ("Content-Type", "text/csv"),
            ("Content-Disposition", "attachment; filename=\"invoices.csv\""),
        ],
        csv_content,
    )
}
