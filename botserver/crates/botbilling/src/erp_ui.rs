use axum::{
    extract::State,
    http::HeaderMap,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use diesel::prelude::*;
use std::sync::Arc;

use crate::api::BillingApiState;

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_empty(msg: &str) -> String {
    format!("<div class=\"empty-state\"><p>{}</p></div>", html_escape(msg))
}

/// Resolve the bot id for the caller's workspace. The caller's branch comes
/// from the JWT (or the user→org binding fallback); the bot is the branch's
/// default bot. All ERP/GL tables scope by `bot_id`, so every query must
/// filter by it to enforce data isolation between branches/tenants.
fn resolve_bot_id(
    pool: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>,
    headers: &HeaderMap,
    get_default_bot: &Option<crate::GetDefaultBotFn>,
) -> uuid::Uuid {
    let Ok(mut conn) = pool.get() else {
        return uuid::Uuid::nil();
    };
    let branch_id = crate::scope::branch_from_jwt(headers, &mut conn)
        .unwrap_or_else(|| crate::get_bot_context(pool, get_default_bot));
    if branch_id == uuid::Uuid::nil() {
        return uuid::Uuid::nil();
    }
    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct BotIdRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: uuid::Uuid,
    }
    diesel::sql_query(
        "SELECT id FROM bots WHERE branch_id = $1 \
         ORDER BY is_default_for_branch DESC, created_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .get_result::<BotIdRow>(&mut conn)
    .map(|r| r.id)
    .unwrap_or_else(|e| {
        tracing::error!("Failed to resolve bot id for branch: {e}");
        uuid::Uuid::nil()
    })
}

pub fn configure_erp_ui_routes() -> Router<Arc<BillingApiState>> {
    Router::new()
        .route("/api/ui/billing/inventory", get(handle_inventory))
        .route("/api/ui/billing/gl/accounts", get(handle_gl_accounts))
        .route("/api/ui/billing/gl/balance-sheet", get(handle_gl_balance_sheet))
        .route("/api/ui/billing/gl/income-statement", get(handle_gl_income_statement))
        .route("/api/ui/billing/procurement", get(handle_procurement))
        .route("/api/ui/billing/procurement/orders", get(handle_procurement_orders))
}

async fn handle_inventory(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let conn = state.pool.clone();
    let bot_id = resolve_bot_id(&conn, &headers, &state.get_default_bot);

    let rows = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("DB connection error: {e}");
                return Vec::new();
            }
        };
        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            sku: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
            #[diesel(sql_type = diesel::sql_types::Double)]
            quantity: f64,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Double>)]
            unit_cost: Option<f64>,
            #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Text>)]
            location: Option<String>,
        }
        diesel::sql_query(
            "SELECT sku, name, quantity, unit_cost, location FROM inventory_items \
             WHERE bot_id = $1 ORDER BY name ASC LIMIT 200",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .load::<Row>(&mut db_conn)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load inventory: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(render_empty("No inventory items yet"));
    }

    let mut html = String::from(
        "<table class=\"billing-table\"><thead><tr><th>SKU</th><th>Name</th><th>Qty</th><th>Unit Cost</th><th>Location</th></tr></thead><tbody>",
    );
    for r in &rows {
        html.push_str(&format!(
            "<tr><td>{sku}</td><td>{name}</td><td>{qty}</td><td>{cost}</td><td>{loc}</td></tr>",
            sku = html_escape(&r.sku),
            name = html_escape(&r.name),
            qty = r.quantity,
            cost = r
                .unit_cost
                .map(|c| format!("${:.2}", c))
                .unwrap_or_else(|| "-".to_string()),
            loc = html_escape(r.location.as_deref().unwrap_or("-")),
        ));
    }
    html.push_str("</tbody></table>");
    Html(html)
}

async fn handle_procurement(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let conn = state.pool.clone();
    let bot_id = resolve_bot_id(&conn, &headers, &state.get_default_bot);

    let rows = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("DB connection error: {e}");
                return Vec::new();
            }
        };
        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            po_number: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            vendor_name: String,
            #[diesel(sql_type = diesel::sql_types::Double)]
            total_amount: f64,
            #[diesel(sql_type = diesel::sql_types::Text)]
            status: String,
            #[diesel(sql_type = diesel::sql_types::Timestamptz)]
            created_at: chrono::DateTime<chrono::Utc>,
        }
        diesel::sql_query(
            "SELECT po_number, vendor_name, total_amount, status, created_at \
             FROM purchase_orders WHERE bot_id = $1 ORDER BY created_at DESC LIMIT 200",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .load::<Row>(&mut db_conn)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load procurement: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(render_empty("No purchase orders yet"));
    }

    let mut html = String::from(
        "<table class=\"billing-table\"><thead><tr><th>PO #</th><th>Vendor</th><th>Total</th><th>Status</th><th>Created</th></tr></thead><tbody>",
    );
    for r in &rows {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            html_escape(&r.po_number),
            html_escape(&r.vendor_name),
            format!("${:.2}", r.total_amount),
            html_escape(&r.status),
            r.created_at.format("%Y-%m-%d"),
        ));
    }
    html.push_str("</tbody></table>");
    Html(html)
}

async fn handle_procurement_orders(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    handle_procurement(State(state), headers).await
}

async fn handle_gl_accounts(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let conn = state.pool.clone();
    let bot_id = resolve_bot_id(&conn, &headers, &state.get_default_bot);

    let rows = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("DB connection error: {e}");
                return Vec::new();
            }
        };
        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            code: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            name: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            account_type: String,
        }
        diesel::sql_query(
            "SELECT code, name, account_type FROM gl_accounts \
             WHERE bot_id = $1 ORDER BY code ASC LIMIT 200",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .load::<Row>(&mut db_conn)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load GL accounts: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(render_empty("No general ledger accounts yet"));
    }

    let mut html = String::from(
        "<table class=\"billing-table\"><thead><tr><th>Code</th><th>Account</th><th>Type</th></tr></thead><tbody>",
    );
    for r in &rows {
        html.push_str(&format!(
            "<tr><td>{code}</td><td>{name}</td><td>{atype}</td></tr>",
            code = html_escape(&r.code),
            name = html_escape(&r.name),
            atype = html_escape(&r.account_type),
        ));
    }
    html.push_str("</tbody></table>");
    Html(html)
}

async fn handle_gl_balance_sheet(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let conn = state.pool.clone();
    let bot_id = resolve_bot_id(&conn, &headers, &state.get_default_bot);

    let rows = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("DB connection error: {e}");
                return Vec::new();
            }
        };
        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            account_name: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            account_type: String,
            #[diesel(sql_type = diesel::sql_types::Double)]
            balance: f64,
        }
        diesel::sql_query(
            "SELECT a.name AS account_name, a.account_type AS account_type, \
             COALESCE(SUM(CASE WHEN l.debit > 0 THEN l.debit ELSE -l.credit END), 0) AS balance \
             FROM gl_accounts a \
             LEFT JOIN gl_journal_lines l ON l.account_id = a.id \
             WHERE a.bot_id = $1 AND a.account_type IN ('Asset', 'Liability', 'Equity') \
             GROUP BY a.id, a.name, a.account_type ORDER BY a.code ASC",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .load::<Row>(&mut db_conn)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load balance sheet: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(render_empty("No balance sheet data yet"));
    }

    let mut html = String::from(
        "<table class=\"billing-table\"><thead><tr><th>Account</th><th>Type</th><th>Balance</th></tr></thead><tbody>",
    );
    for r in &rows {
        html.push_str(&format!(
            "<tr><td>{name}</td><td>{atype}</td><td>{balance}</td></tr>",
            name = html_escape(&r.account_name),
            atype = html_escape(&r.account_type),
            balance = format!("${:.2}", r.balance),
        ));
    }
    html.push_str("</tbody></table>");
    Html(html)
}

async fn handle_gl_income_statement(State(state): State<Arc<BillingApiState>>, headers: HeaderMap) -> impl IntoResponse {
    let conn = state.pool.clone();
    let bot_id = resolve_bot_id(&conn, &headers, &state.get_default_bot);

    let rows = tokio::task::spawn_blocking(move || {
        let mut db_conn = match conn.get() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("DB connection error: {e}");
                return Vec::new();
            }
        };
        #[derive(diesel::QueryableByName)]
        #[diesel(check_for_backend(diesel::pg::Pg))]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Text)]
            account_name: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            account_type: String,
            #[diesel(sql_type = diesel::sql_types::Double)]
            balance: f64,
        }
        diesel::sql_query(
            "SELECT a.name AS account_name, a.account_type AS account_type, \
             COALESCE(SUM(CASE WHEN l.debit > 0 THEN l.debit ELSE -l.credit END), 0) AS balance \
             FROM gl_accounts a \
             LEFT JOIN gl_journal_lines l ON l.account_id = a.id \
             WHERE a.bot_id = $1 AND a.account_type IN ('Revenue', 'Expense') \
             GROUP BY a.id, a.name, a.account_type ORDER BY a.code ASC",
        )
        .bind::<diesel::sql_types::Uuid, _>(bot_id)
        .load::<Row>(&mut db_conn)
        .unwrap_or_else(|e| {
            tracing::error!("Failed to load income statement: {e}");
            Vec::new()
        })
    })
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return Html(render_empty("No income statement data yet"));
    }

    let mut html = String::from(
        "<table class=\"billing-table\"><thead><tr><th>Account</th><th>Type</th><th>Amount</th></tr></thead><tbody>",
    );
    for r in &rows {
        html.push_str(&format!(
            "<tr><td>{name}</td><td>{atype}</td><td>{balance}</td></tr>",
            name = html_escape(&r.account_name),
            atype = html_escape(&r.account_type),
            balance = format!("${:.2}", r.balance),
        ));
    }
    html.push_str("</tbody></table>");
    Html(html)
}
