use super::*;

/// Archives invoice-like drive files (issue #723): lists files under the
/// `{bot}.gbdrive/{source}` folder, moves each into `{destination}/` so the
/// inbox stays clean, and returns a summary of archived files. Source may be
/// left empty to scan the whole drive; destination is required.
pub(super) async fn archive_drive_files(
    state: &Arc<AppState>,
    bot_uuid: &Uuid,
    source: &str,
    destination: &str,
) -> Result<Value, String> {
    let drive = state
        .drive
        .clone()
        .ok_or_else(|| "drive unavailable".to_string())?;
    let loc = catalog_drive_location(state, bot_uuid)?;
    let bucket = loc.bucket.clone();
    let src_prefix = if source.is_empty() {
        loc.key_prefix.clone()
    } else {
        loc.key_for(&normalize_drive_path(source)?)
    };
    let dest_rel = normalize_drive_path(destination)?;
    let dest_prefix = loc.key_for(&dest_rel);
    let keys = drive
        .list_objects_with_metadata(&bucket, Some(&src_prefix))
        .await
        .map_err(|e| format!("drive list error: {e}"))?;

    let mut archived = 0i32;
    let mut skipped = 0i32;
    for obj in keys {
        let key = obj.key.clone();
        // Do not re-process a file already inside the destination folder.
        if key.starts_with(&format!("{dest_prefix}/")) {
            skipped += 1;
            continue;
        }
        // Resolve the relative file name (the final path segment).
        let file_name = key.rsplit('/').next().unwrap_or(&key);
        let to_key = format!("{dest_prefix}/{file_name}");
        let bytes = drive
            .get_object(&bucket, &key)
            .await
            .map_err(|e| format!("read failed at {key}: {e}"))?;
        let put = drive.put_object(&bucket, &to_key, bytes, None).await;
        match put {
            Ok(()) => {
                archived += 1;
                log::info!("api_call drive.archive -> {to_key}");
            }
            Err(e) => {
                skipped += 1;
                log::error!("drive.archive skip {key}: {e}");
            }
        }
    }

    Ok(json!({
        "success": true,
        "bucket": bucket,
        "archived": archived,
        "skipped": skipped,
        "destination": destination,
    }))
}
/// Payroll diagnosis (issue #724): aggregates outgoing `billing_invoices` of
/// the branch by month and returns a compact financial summary the LLM can
/// present to the user.
pub(super) async fn payroll_diagnosis(
    state: &Arc<AppState>,
    bot_uuid: &Uuid,
    period: Option<&str>,
) -> Result<Value, String> {
    let branch = branch_scope(state, bot_uuid)?;

    #[derive(diesel::QueryableByName)]
    struct PayRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        month_key: String,
        #[diesel(sql_type = diesel::sql_types::Int8)]
        count: i64,
        #[diesel(sql_type = diesel::sql_types::Numeric)]
        total: bigdecimal::BigDecimal,
    }

    let mut conn = state.conn.get().map_err(|e| format!("DB error: {e}"))?;

    let filter = "SELECT to_char(issue_date, 'YYYY-MM') AS month_key, \
                count(*)::bigint AS count, \
                coalesce(sum(total), 0) AS total \
         FROM billing_invoices WHERE branch_id = $1 AND to_char(issue_date, 'YYYY-MM') LIKE $2 \
         GROUP BY month_key ORDER BY month_key DESC LIMIT 12";
    let all = "SELECT to_char(issue_date, 'YYYY-MM') AS month_key, \
                count(*)::bigint AS count, \
                coalesce(sum(total), 0) AS total \
         FROM billing_invoices WHERE branch_id = $1 \
         GROUP BY month_key ORDER BY month_key DESC LIMIT 12";

    let rows: Vec<PayRow>; 
    if let Some(p) = period {
        let month_prefix = format!("{p}%");
        rows = diesel::sql_query(filter)
            .bind::<diesel::sql_types::Uuid, _>(branch)
            .bind::<diesel::sql_types::Text, _>(month_prefix)
            .load(&mut conn)
            .map_err(|e| format!("Query error: {e}"))?;
    } else {
        rows = diesel::sql_query(all)
            .bind::<diesel::sql_types::Uuid, _>(branch)
            .load(&mut conn)
            .map_err(|e| format!("Query error: {e}"))?;
    }

    let months: Vec<Value> = rows
        .into_iter()
        .map(|r| json!({ "month": r.month_key, "invoices": r.count, "total": r.total }))
        .collect();
    Ok(json!({
        "branch_id": branch,
        "months": months,
        "summary": if months.is_empty() { "no invoices recorded".to_string() } else { "monthly invoice totals above; use them as the payroll ledger basis".to_string() },
    }))
}
