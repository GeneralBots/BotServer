//! Drive object seeding for the fiscal test scenarios (issues #722/#723/#724).
//!
//! Writes real MinIO objects into the default bot's bucket so the chat/API
//! flows can discover them exactly like a production user's Drive:
//!
//!   * `faturas/`  — invoice folder used by the guided upload flow (#723)
//!   * `financeiro/` — cash-flow CSV files used by the banking import (#724)
//!
//! All inserts are guarded by `object_exists`, so re-running is safe.

use botlib::traits::DriveRepository;
use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::Text;/// Name, branch slug and org slug of the default bot.
///
/// The slugs are not optional decoration: the org decides whether the files
/// land in the bot's own `{bot}.gbai` bucket or in the shared `{slug}.gborg`
/// workspace, and the branch names the `{branch}.gbai/` workspace inside that
/// bucket — so neither can be built from the bot name alone.
/// Bot name with its branch and org slugs.
type DefaultBotSlugs = (String, Option<String>, Option<String>);

fn resolve_default_bot(
    conn: &mut diesel::PgConnection,
) -> Result<Option<DefaultBotSlugs>, String> {
    #[derive(diesel::QueryableByName)]
    struct BotRow {
        #[diesel(sql_type = Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
        branch_slug: Option<String>,
        #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
        org_slug: Option<String>,
    }
    let row: Option<BotRow> = sql_query(
        "SELECT b.name, br.slug AS branch_slug, o.slug AS org_slug
         FROM bots b
         LEFT JOIN branches br ON br.id = b.branch_id
         LEFT JOIN organizations o ON o.org_id = b.org_id
         WHERE b.is_default_for_branch = true
         ORDER BY b.created_at ASC LIMIT 1",
    )
    .get_result(conn)
    .ok();

    Ok(row.map(|r| (r.name, r.branch_slug, r.org_slug)))
}

/// Builds a cash-flow CSV with entries for the given month (so diagnosis
/// filters by the current month prefix correctly). Uses the real-world
/// Brazilian column names (data/historico/valor/tipo) that the import
/// parsers accept.
fn build_cashflow_csv(year: u32, month_idx: u32) -> String {
    let last_day = if month_idx == 2 { 28 } else { 30 };
    let d = |day: u32| format!("{year:04}-{month_idx:02}-{day:02}");
    format!(
        "data,historico,valor,tipo\n\
         {},\"Client contract\",5000.00,receita\n\
         {},\"Consulting hours\",2500.00,receita\n\
         {},\"Cloud subscription\",-1200.00,despesa\n\
         {},\"Office rent\",-1800.00,despesa\n\
         {},\"Telecom\",-{last_day}.00,despesa\n\
         {},\"Marketing\",-600.00,despesa\n",
        d(2),
        d(5),
        d(8),
        d(12),
        d(15),
        d(20),
    )
}

/// Seeds the invoice folder and cash-flow spreadsheets of the default bot.
pub async fn seed_drive_objects(
    pool: &botcore::shared::utils::DbPool,
    drive: &dyn DriveRepository,
) -> Result<(), String> {
    let mut conn = pool.get().map_err(|e| format!("Pool error: {e}"))?;
    let (bot_name, branch_slug, org_slug) =
        resolve_default_bot(&mut conn)?.ok_or("No default bot found")?;
    drop(conn);

    // Resolved rather than formatted: for an org-hosted bot this is the org
    // workspace bucket plus the bot's key prefix inside its branch workspace.
    let location = botbasic_core::utils::resolve_bot_drive_location(
        &bot_name,
        branch_slug.as_deref(),
        org_slug.as_deref(),
    );
    let prefix = location.drive_prefix();
    let bucket = location.bucket;

    drive
        .create_bucket_if_not_exists(&bucket)
        .await
        .map_err(|e| format!("create bucket {bucket}: {e}"))?;

    let now = chrono::Utc::now();
    let current_month = now.format("%Y-%m").to_string();
    let prev = now
        .checked_sub_months(chrono::Months::new(1))
        .unwrap_or(now);
    let prev_month = prev.format("%Y-%m").to_string();
    let year = now.format("%Y").to_string().parse::<u32>().unwrap_or(2026);
    let month_idx = now.format("%m").to_string().parse::<u32>().unwrap_or(8);
    let prev_year = prev.format("%Y").to_string().parse::<u32>().unwrap_or(2026);
    let prev_month_idx = prev.format("%m").to_string().parse::<u32>().unwrap_or(7);

    let faturas_marker = format!("{prefix}faturas/.keep");
    if !drive.object_exists(&bucket, &faturas_marker).await.unwrap_or(false) {
        drive
            .put_object(&bucket, &faturas_marker, Vec::new(), Some("text/plain"))
            .await
            .map_err(|e| format!("create faturas folder: {e}"))?;
    }

    let invoice_key = format!("{prefix}faturas/telefonia-{current_month}.pdf");
    if !drive.object_exists(&bucket, &invoice_key).await.unwrap_or(false) {
        drive
            .put_object(
                &bucket,
                &invoice_key,
                format!("%PDF-1.4 sample invoice {current_month}").into_bytes(),
                Some("application/pdf"),
            )
            .await
            .map_err(|e| format!("seed invoice: {e}"))?;
    }

    let financeiro_marker = format!("{prefix}financeiro/.keep");
    if !drive.object_exists(&bucket, &financeiro_marker).await.unwrap_or(false) {
        drive
            .put_object(&bucket, &financeiro_marker, Vec::new(), Some("text/plain"))
            .await
            .map_err(|e| format!("create financeiro folder: {e}"))?;
    }

    let finance_key = format!("{prefix}financeiro/fluxo-caixa-{current_month}.csv");
    if !drive.object_exists(&bucket, &finance_key).await.unwrap_or(false) {
        drive
            .put_object(
                &bucket,
                &finance_key,
                build_cashflow_csv(year, month_idx).into_bytes(),
                Some("text/csv"),
            )
            .await
            .map_err(|e| format!("seed current cashflow: {e}"))?;
    }

    let prev_key = format!("{prefix}financeiro/fluxo-caixa-{prev_month}.csv");
    if !drive.object_exists(&bucket, &prev_key).await.unwrap_or(false) {
        drive
            .put_object(
                &bucket,
                &prev_key,
                build_cashflow_csv(prev_year, prev_month_idx).into_bytes(),
                Some("text/csv"),
            )
            .await
            .map_err(|e| format!("seed previous cashflow: {e}"))?;
    }

    Ok(())
}
