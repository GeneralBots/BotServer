use super::*;

pub(super) fn error_response(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({"error": msg})),
    )
}
pub(super) fn internal_error(msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({"error": msg})),
    )
}
/// Builds the `DEFAULT ...` clause for a column. A UUID primary key with no
/// explicit default gets `gen_random_uuid()` so the first row insert (which
/// omits the auto-generated PK) doesn't fail with a not-null violation.
pub(super) fn column_default(col: &ColumnDef, data_type: &str) -> String {
    col.default
        .as_ref()
        .map(|d| format!(" DEFAULT {d}"))
        .unwrap_or_else(|| {
            let is_uuid_pk = col.primary_key.unwrap_or(false) && data_type.eq_ignore_ascii_case("uuid");
            if is_uuid_pk {
                " DEFAULT gen_random_uuid()".to_string()
            } else {
                String::new()
            }
        })
}
pub(super) fn extract_bot_id(headers: &HeaderMap) -> Result<uuid::Uuid, (StatusCode, Json<serde_json::Value>)> {
    if let Some(v) = headers.get("X-Bot-Id").and_then(|v| v.to_str().ok()) {
        let bot_id = uuid::Uuid::parse_str(v)
            .map_err(|_| error_response("Invalid X-Bot-Id header"))?;
        authorize_bot_access(headers, bot_id)?;
        return Ok(bot_id);
    }

    // Fall back to the default bot when no header is provided so the
    // database app works without an explicit bot context.
    let pool = db::pool().map_err(|(code, msg)| (code, Json(serde_json::json!({"error": msg}))))?;
    let mut conn = pool
        .get()
        .map_err(|e| internal_error(&format!("Main DB connection error: {e}")))?;
    let (bot_id, _) = botcore::bot::get_default_bot(&mut conn);
    Ok(bot_id)
}
/// #1386 — Vibe project context for the Database pane. `X-Vibe-Project: <uuid>`
/// selects the selected Vibe project's OWN database pair
/// (`app_{branch}_{project}` / `..._dev`), exactly like the Chat and Terminal
/// panes contextualize on the same selection. Every project kind owns its
/// pair (bots, websites and apps). The project must belong to the caller's
/// branch (resolved from the JWT claims — never client input), and `X-Db-Env`
/// keeps switching between the project's production database and its dev twin.
/// Project databases are created on demand so a fresh project can be inspected
/// before its first run.
pub(super) fn resolve_project_pool(
    project_id: uuid::Uuid,
    headers: &HeaderMap,
) -> Result<botcore::shared::utils::DbPool, (StatusCode, Json<serde_json::Value>)> {
    let pool = db::pool().map_err(|(code, msg)| (code, Json(serde_json::json!({"error": msg}))))?;
    let mut conn = pool
        .get()
        .map_err(|e| internal_error(&format!("Main DB connection error: {e}")))?;

    #[derive(diesel::QueryableByName)]
    struct ProjectRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        branch_id: uuid::Uuid,
    }
    let project: ProjectRow = diesel::sql_query(
        "SELECT name, branch_id FROM vibe_projects WHERE id = $1",
    )
    .bind::<diesel::sql_types::Uuid, _>(project_id)
    .get_result(&mut conn)
    .map_err(|_| error_response("Vibe project not found"))?;

    // Tenant check: the caller's branch must own the project (nil branch =
    // global/super-admin scope, same rule as authorize_bot_access).
    if let Some(branch) = botcore::shared::tenant::branch_from_claims(headers) {
        if branch != uuid::Uuid::nil() && branch != project.branch_id {
            return Err((
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error": "Project not accessible in this workspace"})),
            ));
        }
    }

    let env = headers
        .get("X-Db-Env")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("production");
    let db_name =
        botcore::project_db::project_database_name(project.branch_id, &project.name, env);
    botcore::project_db::validate_db_name(&db_name)
        .map_err(|e| error_response(&format!("Invalid project database: {e}")))?;

    // #1447 S1 — the identifier is interpolated into pg_database text and
    // DDL below; the strict allowlist check runs at each interpolation site.
    botcore::project_db::assert_safe_db_name(&db_name)
        .map_err(|e| error_response(&format!("Invalid project database: {e}")))?;

    let exists: bool = diesel::sql_query(format!(
        "SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = '{db_name}') AS exists"
    ))
    .get_result::<ExistsRow>(&mut conn)
    .map(|r| r.exists)
    .map_err(|e| internal_error(&format!("Check project database: {e}")))?;
    if !exists {
        botcore::project_db::assert_safe_db_name(&db_name)
            .map_err(|e| error_response(&format!("Invalid project database: {e}")))?;
        diesel::sql_query(format!("CREATE DATABASE {db_name}"))
            .execute(&mut conn)
            .map_err(|e| internal_error(&format!("Create project database: {e}")))?;
        log::info!("apps database: created project database {db_name} on demand");
    }
    drop(conn);

    let url = botcore::project_db::database_url_for(&db_name);
    let manager = diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new(url);
    diesel::r2d2::Pool::builder()
        .max_size(3)
        .build(manager)
        .map_err(|e| internal_error(&format!("Project database pool: {e}")))
}
/// Process-lifetime cache of the resolved per-bot database URL, so the
/// `bots.database_name` lookup does not run on every schema/table/query
/// request. The resolved name is stable within a process lifetime, so a
/// static cache is safe here.
pub(super) fn bot_db_url_cache() -> &'static std::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>> {
    static CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<uuid::Uuid, String>>> =
        std::sync::OnceLock::new();
    CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}
/// Tenant authorization for a client-supplied bot id (issue #850, following
/// the #734 pattern in botbanking::cashflow::resolve_bot_scope): a caller may
/// only address a bot that belongs to their workspace branch. The caller's
/// branch is resolved exclusively from the server-minted JWT `branch_id`
/// claim — never from client input. Callers without a branch claim
/// (global/super-admin tokens) and the legacy nil branch are unrestricted.
pub(super) fn authorize_bot_access(
    headers: &HeaderMap,
    bot_id: uuid::Uuid,
) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    let caller_branch = match botcore::shared::tenant::branch_from_claims(headers) {
        Some(branch) if branch != uuid::Uuid::nil() => branch,
        _ => return Ok(()),
    };

    let pool = db::pool().map_err(|(code, msg)| (code, Json(serde_json::json!({"error": msg}))))?;
    let mut conn = pool
        .get()
        .map_err(|e| internal_error(&format!("Main DB connection error: {e}")))?;

    #[derive(diesel::QueryableByName)]
    struct BotRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        branch_id: uuid::Uuid,
    }
    let row: Option<BotRow> = diesel::sql_query(
        "SELECT branch_id FROM bots WHERE id = $1 AND is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .get_result(&mut conn)
    .ok();

    match row {
        Some(r) if r.branch_id == caller_branch => Ok(()),
        Some(_) => Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Bot not accessible in this workspace"})),
        )),
        None => Err(error_response("Bot not found or inactive")),
    }
}
pub(super) fn get_bot_pool(
    state: &AppState,
    bot_id: uuid::Uuid,
) -> Result<botcore::shared::utils::DbPool, (StatusCode, Json<serde_json::Value>)> {
    state
        .bot_database_manager
        .get_bot_pool(bot_id)
        .ok_or_else(|| error_response("Bot database not available"))
}
/// #1386 — environment override for the DB dialog. `X-Db-Env: dev` points
/// the whole dialog at the bot's dev-twin database (`bot_..._dev`, the
/// database a `{bot}-dev` twin bot would own) instead of the production one,
/// so a developer can browse both environments from the same UI. The twin
/// database is created on demand; production stays untouched.
pub(super) fn get_bot_pool_for_env(
    state: &AppState,
    bot_id: uuid::Uuid,
    env: &str,
) -> Result<botcore::shared::utils::DbPool, (StatusCode, Json<serde_json::Value>)> {
    let is_dev = matches!(env.to_ascii_lowercase().as_str(), "dev" | "test" | "development");
    if !is_dev {
        return get_bot_pool(state, bot_id);
    }
    // Resolve the bot's own database name, derive the `_dev` twin name from
    // it, and open a dedicated pool. Cached per process (same rationale as
    // the resolved-name cache above).
    let pool = db::pool().map_err(|(code, msg)| (code, Json(serde_json::json!({"error": msg}))))?;
    let mut conn = pool
        .get()
        .map_err(|e| internal_error(&format!("Main DB connection error: {e}")))?;

    #[derive(diesel::QueryableByName)]
    struct BotDbRow {
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Varchar>)]
        database_name: Option<String>,
    }
    let row: BotDbRow = diesel::sql_query(
        "SELECT database_name FROM bots WHERE id = $1 AND is_active = true",
    )
    .bind::<diesel::sql_types::Uuid, _>(bot_id)
    .get_result(&mut conn)
    .map_err(|_| error_response("Bot not found or inactive"))?;

    let prod_db = row
        .database_name
        .ok_or_else(|| error_response("Bot has no database yet"))?;
    let dev_db = if prod_db.ends_with("_dev") {
        prod_db.clone()
    } else {
        format!("{prod_db}_dev")
    };

    // #1447 S1 — identifier-safe by construction (derived from an existing
    // bot DB name); the strict allowlist check still runs at each
    // interpolation site below.
    botcore::project_db::assert_safe_db_name(&dev_db)
        .map_err(|e| error_response(&format!("Invalid dev database: {e}")))?;

    let exists: bool = diesel::sql_query(format!(
        "SELECT EXISTS (SELECT 1 FROM pg_database WHERE datname = '{dev_db}') AS exists"
    ))
    .get_result::<ExistsRow>(&mut conn)
    .map(|r| r.exists)
    .map_err(|e| internal_error(&format!("Check dev database: {e}")))?;
    if !exists {
        botcore::project_db::assert_safe_db_name(&dev_db)
            .map_err(|e| error_response(&format!("Invalid dev database: {e}")))?;
        diesel::sql_query(format!("CREATE DATABASE {dev_db}"))
            .execute(&mut conn)
            .map_err(|e| internal_error(&format!("Create dev database: {e}")))?;
    }
    drop(conn);

    let base = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/botserver".to_string());
    let url = match base.rfind('/') {
        Some(pos) => format!("{}/{}{}", &base[..pos], dev_db, base[pos..].split('?').nth(1).map(|q| format!("?{q}")).unwrap_or_default()),
        None => format!("{base}/{dev_db}"),
    };
    let manager = diesel::r2d2::ConnectionManager::<diesel::PgConnection>::new(url);
    diesel::r2d2::Pool::builder()
        .max_size(3)
        .build(manager)
        .map_err(|e| internal_error(&format!("Dev database pool: {e}")))
}
