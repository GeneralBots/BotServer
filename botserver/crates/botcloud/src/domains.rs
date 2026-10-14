use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

use crate::api::{get_branch_id_from_jwt, is_super_admin};
use crate::SaasService;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema_ext::bot_domains)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct BotDomain {
    pub id: Uuid,
    pub domain: String,
    pub bot_id: Uuid,
    pub org_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDomainBody {
    pub domain: String,
    pub bot_id: Uuid,
    pub org_id: Option<Uuid>,
    pub branch_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDomainBody {
    pub domain: Option<String>,
    pub bot_id: Option<Uuid>,
}

/// Resolve scope filter: admin sees all, user sees only their branch.
fn get_scope_filter(
    headers: &HeaderMap,
    conn: &mut diesel::PgConnection,
) -> Result<(bool, Option<Uuid>), (StatusCode, String)> {
    let admin = is_super_admin(headers, conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    let user_branch = if admin {
        None
    } else {
        get_branch_id_from_jwt(headers, conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?
    };
    Ok((admin, user_branch))
}

/// `GET /api/cloud/domains` — list domain mappings scoped to user's branch (or all for admin)
pub async fn list_domains(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let (admin, user_branch) = get_scope_filter(&headers, &mut conn)?;

    use crate::schema_ext::bot_domains::dsl::{bot_domains, created_at, branch_id as br_id};

    let domains = if admin {
        bot_domains
            .order(created_at.desc())
            .load::<BotDomain>(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query domains: {e}")))?
    } else if let Some(bid) = user_branch {
        bot_domains
            .filter(br_id.eq(bid))
            .order(created_at.desc())
            .load::<BotDomain>(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query domains: {e}")))?
    } else {
        Vec::new()
    };

    Ok(Json(serde_json::json!({ "domains": domains })))
}

/// `POST /api/cloud/domains` — create a domain mapping
pub async fn create_domain(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
    Json(body): Json<CreateDomainBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    // Domain creation is admin-only (documented contract: JWT + super admin).
    // Never allow a non-admin caller to mint an arbitrary domain→bot mapping
    // with attacker-controlled org_id/branch_id.
    let (admin, user_branch) = get_scope_filter(&headers, &mut conn)?;
    if !admin {
        return Err((StatusCode::FORBIDDEN, "Administrator access required".to_string()));
    }

    let domain = body.domain.trim().to_lowercase();
    if domain.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "Domain is required".to_string()));
    }

    let now = Utc::now();
    let id = Uuid::new_v4();

    use crate::schema_ext::bot_domains::dsl::{bot_domains, id as did, domain as dcol, bot_id as bcol, org_id as ocol, branch_id as brcol, created_at as cat, updated_at as uat};

    diesel::insert_into(bot_domains)
        .values((
            did.eq(id),
            dcol.eq(&domain),
            bcol.eq(body.bot_id),
            ocol.eq(body.org_id),
            brcol.eq(body.branch_id.or(user_branch)),
            cat.eq(now),
            uat.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::CONFLICT, format!("Failed to create domain mapping: {e}")))?;

    Ok(Json(serde_json::json!({
        "status": "created",
        "id": id,
        "domain": domain,
        "bot_id": body.bot_id,
    })))
}

/// `PUT /api/cloud/domains/{id}` — update a domain mapping
pub async fn update_domain(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
    Path(domain_id): Path<Uuid>,
    Json(body): Json<UpdateDomainBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let (admin, user_branch) = get_scope_filter(&headers, &mut conn)?;

    let now = Utc::now();

    if let Some(ref d) = body.domain {
        let d = d.trim().to_lowercase();
        if !d.is_empty() {
            if admin {
                diesel::sql_query("UPDATE bot_domains SET domain = $1, updated_at = $2 WHERE id = $3")
                    .bind::<diesel::sql_types::Text, _>(&d)
                    .bind::<diesel::sql_types::Timestamptz, _>(now)
                    .bind::<diesel::sql_types::Uuid, _>(domain_id)
                    .execute(&mut conn)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update domain: {e}")))?;
            } else if let Some(bid) = user_branch {
                diesel::sql_query("UPDATE bot_domains SET domain = $1, updated_at = $2 WHERE id = $3 AND (branch_id = $4 OR branch_id IS NULL)")
                    .bind::<diesel::sql_types::Text, _>(&d)
                    .bind::<diesel::sql_types::Timestamptz, _>(now)
                    .bind::<diesel::sql_types::Uuid, _>(domain_id)
                    .bind::<diesel::sql_types::Uuid, _>(bid)
                    .execute(&mut conn)
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update domain: {e}")))?;
            } else {
                return Err((StatusCode::FORBIDDEN, "Not authorized".to_string()));
            }
        }
    }

    if let Some(b) = body.bot_id {
        if admin {
            diesel::sql_query("UPDATE bot_domains SET bot_id = $1, updated_at = $2 WHERE id = $3")
                .bind::<diesel::sql_types::Uuid, _>(b)
                .bind::<diesel::sql_types::Timestamptz, _>(now)
                .bind::<diesel::sql_types::Uuid, _>(domain_id)
                .execute(&mut conn)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update bot_id: {e}")))?;
        } else if let Some(bid) = user_branch {
            diesel::sql_query("UPDATE bot_domains SET bot_id = $1, updated_at = $2 WHERE id = $3 AND (branch_id = $4 OR branch_id IS NULL)")
                .bind::<diesel::sql_types::Uuid, _>(b)
                .bind::<diesel::sql_types::Timestamptz, _>(now)
                .bind::<diesel::sql_types::Uuid, _>(domain_id)
                .bind::<diesel::sql_types::Uuid, _>(bid)
                .execute(&mut conn)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update bot_id: {e}")))?;
        } else {
            return Err((StatusCode::FORBIDDEN, "Not authorized".to_string()));
        }
    }

    Ok(Json(serde_json::json!({ "status": "updated" })))
}

/// `DELETE /api/cloud/domains/{id}` — delete a domain mapping
pub async fn delete_domain(
    State(service): State<Arc<SaasService>>,
    headers: HeaderMap,
    Path(domain_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let (admin, user_branch) = get_scope_filter(&headers, &mut conn)?;

    let deleted = if admin {
        diesel::sql_query("DELETE FROM bot_domains WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(domain_id)
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete: {e}")))?
    } else if let Some(bid) = user_branch {
        diesel::sql_query("DELETE FROM bot_domains WHERE id = $1 AND (branch_id = $2 OR branch_id IS NULL)")
            .bind::<diesel::sql_types::Uuid, _>(domain_id)
            .bind::<diesel::sql_types::Uuid, _>(bid)
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete: {e}")))?
    } else {
        return Err((StatusCode::FORBIDDEN, "Not authorized".to_string()));
    };

    if deleted == 0 {
        return Err((StatusCode::NOT_FOUND, "Domain mapping not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "status": "deleted" })))
}

/// Try to match the host against a wildcard pattern like `%.example.com`.
fn wildcard_matches(pattern: &str, host: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix("\\\\*.") {
        host.ends_with(&format!(".{suffix}"))
    } else if let Some(suffix) = pattern.strip_prefix("*.") {
        host.ends_with(&format!(".{suffix}"))
    } else {
        false
    }
}

/// Platform domain for SaaS subdomain mode (parameterized for self-hosting).
/// `{bot}.{platform_domain}` serves the bot named `{bot}` without requiring a
/// `bot_domains` row. Override with the `GB_PLATFORM_DOMAIN` env var when
/// self-hosting under a different domain.
pub fn platform_domain() -> String {
    std::env::var("GB_PLATFORM_DOMAIN")
        .unwrap_or_else(|_| "generalbots.org".to_string())
        .to_lowercase()
}

/// Reserved platform subdomains that never map to a bot by name — they host
/// platform services (suite entry, store, docs, …) and serve the default bot.
const PLATFORM_RESERVED_SUBDOMAINS: &[&str] = &[
    "chat", "www", "api", "app", "bot", "docs", "store", "cloud", "login",
    "admin", "mail", "smtp", "imap", "ns1", "ns2",
];

#[derive(diesel::QueryableByName)]
struct SubdomainCount {
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    n: i64,
}

/// `GET /api/domains/tls-ask?domain=<host>` — Caddy on-demand TLS decision
/// endpoint (public, unauthenticated). Caddy's `tls on_demand ask` contract:
/// HTTP 200 = obtain the certificate, any 4xx/5xx = refuse. This adapter
/// reuses the platform resolution rules so a freshly published bot gets its
/// certificate with zero infra change: an exact `bot_domains` row (custom
/// domain mapped in Domain Manager), or a `{bot}.{GB_PLATFORM_DOMAIN}`
/// subdomain where `{bot}` is an active bot (platform subdomain mode).
/// Reserved subdomains (chat., www., docs., …) are NOT allowed so wildcard
/// DNS entries for platform infrastructure cannot farm certificates.
pub async fn tls_ask_domain(
    State(service): State<Arc<SaasService>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> StatusCode {
    let Some(host) = params
        .get("domain")
        .map(|h| h.trim().trim_end_matches('.').to_lowercase())
        .filter(|h| !h.is_empty())
    else {
        return StatusCode::FORBIDDEN;
    };
    // Must be a DNS name with at least two labels — never an IP or bare TLD.
    if host.split('.').count() < 2 {
        return StatusCode::FORBIDDEN;
    }
    let mut conn = match service.pool().get() {
        Ok(c) => c,
        Err(_) => return StatusCode::FORBIDDEN,
    };

    use crate::schema_ext::bot_domains::dsl::{bot_domains, domain as dcol};

    // Allow-list 1: explicit custom-domain mapping.
    let mapped: Option<i64> = bot_domains
        .filter(dcol.eq(&host))
        .select(diesel::dsl::count_star())
        .first::<i64>(&mut conn)
        .optional()
        .unwrap_or(None);
    if mapped.unwrap_or(0) > 0 {
        return StatusCode::OK;
    }

    // Allow-list 2: platform subdomain of an active bot.
    let pdomain = platform_domain();
    if let Some(sub) = host.strip_suffix(&format!(".{pdomain}")) {
        let sub = sub.to_lowercase();
        if !sub.is_empty() && !PLATFORM_RESERVED_SUBDOMAINS.contains(&sub.as_str()) {
            let active: Option<i64> = diesel::sql_query(
                "SELECT COUNT(*) AS n FROM bots WHERE LOWER(name) = $1 AND is_active = true",
            )
            .bind::<diesel::sql_types::Text, _>(&sub)
            .get_result::<SubdomainCount>(&mut conn)
            .optional()
            .ok()
            .flatten()
            .map(|r| r.n);
            if active.unwrap_or(0) > 0 {
                return StatusCode::OK;
            }
        }
    }

    StatusCode::FORBIDDEN
}

/// `GET /api/domains/resolve?host=<host>` — resolve hostname to bot name (public).
/// Match order: exact `bot_domains` row → platform subdomain
/// (`{bot}.{GB_PLATFORM_DOMAIN}`) → wildcard `*.domain` patterns.
pub async fn resolve_domain(
    State(service): State<Arc<SaasService>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let host = params.get("host")
        .map(|h| h.trim().to_lowercase())
        .filter(|h| !h.is_empty())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing host parameter".to_string()))?;

    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    use crate::schema_ext::bot_domains::dsl::{bot_domains, domain as dcol, bot_id};

    #[derive(diesel::QueryableByName, Debug)]
    struct BotNameRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }

    // Step 1: exact match
    let result: Option<(Uuid, Option<Uuid>, Option<Uuid>)> = bot_domains
        .filter(dcol.eq(&host))
        .select((bot_id, crate::schema_ext::bot_domains::dsl::org_id, crate::schema_ext::bot_domains::dsl::branch_id))
        .first::<(Uuid, Option<Uuid>, Option<Uuid>)>(&mut conn)
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;

    if let Some((bot_uuid, org_id, branch_id)) = result {
        let bot_name: Option<String> = diesel::sql_query("SELECT name FROM bots WHERE id = $1")
            .bind::<diesel::sql_types::Uuid, _>(bot_uuid)
            .get_result::<BotNameRow>(&mut conn)
            .ok()
            .map(|r| r.name);

        return match bot_name {
            Some(name) => Ok(Json(serde_json::json!({
                "found": true, "bot_id": bot_uuid, "bot_name": name,
                "org_id": org_id, "branch_id": branch_id,
                "match_type": "exact",
            }))),
            None => Ok(Json(serde_json::json!({
                "found": false, "error": "Bot not found for the mapped domain"
            }))),
        };
    }

    // Step 1.5: platform subdomain mode — `{bot}.{GB_PLATFORM_DOMAIN}` serves
    // the bot named `{bot}` with no bot_domains row (SaaS publishing default).
    // Reserved hosts (chat., www., …) fall through so callers use the default bot.
    let pdomain = platform_domain();
    if let Some(sub) = host.strip_suffix(&format!(".{pdomain}")) {
        let sub = sub.to_lowercase();
        if !sub.is_empty() && !PLATFORM_RESERVED_SUBDOMAINS.contains(&sub.as_str()) {
            let row: Option<BotNameRow> = diesel::sql_query(
                "SELECT name FROM bots WHERE LOWER(name) = $1 AND is_active = true LIMIT 1",
            )
            .bind::<diesel::sql_types::Text, _>(sub)
            .get_result(&mut conn)
            .optional()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;
            if let Some(r) = row {
                return Ok(Json(serde_json::json!({
                    "found": true, "bot_name": r.name, "match_type": "platform_subdomain",
                })));
            }
        }
    }

    // Step 2: wildcard match for `*.domain` patterns — fetch all wildcard domains and test
    #[derive(diesel::QueryableByName, Debug)]
    struct WildcardRow {
        #[diesel(sql_type = diesel::sql_types::Text)]
        domain: String,
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        b_id: Uuid,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
        org_id: Option<Uuid>,
        #[diesel(sql_type = diesel::sql_types::Nullable<diesel::sql_types::Uuid>)]
        branch_id: Option<Uuid>,
    }

    let wildcards: Vec<WildcardRow> = diesel::sql_query(
        "SELECT domain, bot_id, org_id, branch_id FROM bot_domains WHERE domain LIKE '\\\\*.%' OR domain LIKE '*.'",
    )
    .load(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Wildcard query: {e}")))?;

    for w in &wildcards {
        if wildcard_matches(&w.domain, &host) {
            let bot_name: Option<String> = diesel::sql_query("SELECT name FROM bots WHERE id = $1")
                .bind::<diesel::sql_types::Uuid, _>(w.b_id)
                .get_result::<BotNameRow>(&mut conn)
                .ok()
                .map(|r| r.name);

            return match bot_name {
                Some(name) => Ok(Json(serde_json::json!({
                    "found": true, "bot_id": w.b_id, "bot_name": name,
                    "org_id": w.org_id, "branch_id": w.branch_id,
                    "match_type": "wildcard",
                }))),
                None => break,
            };
        }
    }

    Ok(Json(serde_json::json!({ "found": false })))
}
