use super::*;

pub(super) fn cached_quota() -> Option<QuotaResponse> {
    let cell = QUOTA_CACHE.get_or_init(|| std::sync::Mutex::new(None));
    // The lock is held only to read a small value — never across an await.
    let guard = cell.lock().ok()?;
    match guard.as_ref() {
        Some((computed_at, quota)) if computed_at.elapsed() < QUOTA_TTL => Some(quota.clone()),
        _ => None,
    }
}
pub(super) fn store_quota(quota: &QuotaResponse) {
    if let Some(cell) = QUOTA_CACHE.get() {
        if let Ok(mut guard) = cell.lock() {
            *guard = Some((Instant::now(), quota.clone()));
        }
    }
}
pub(super) fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
    warn!("drive_handler error ({}): {}", status.as_u16(), msg);
    (status, Json(serde_json::json!({"error": msg})))
}
pub(super) fn resolve_scope_prefix(scope: &FileScope, user_id: &str) -> String {
    match scope {
        FileScope::User => user_scope::resolve_key_prefix(scope, user_id),
        FileScope::Bot => user_scope::resolve_key_prefix(scope, user_id),
    }
}
pub(super) fn resolve_bucket<'a>(
    state: &'a AppState,
    override_bucket: Option<&'a str>,
    scope: &FileScope,
    user_id: Option<&'a str>,
    bot_name: Option<&'a str>,
    user: &AuthenticatedUser,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    if let Some(b) = override_bucket {
        // Tenant isolation (#1401, #1387): a client-supplied bucket name is
        // only honored when it belongs to the caller's own tenant. PLATFORM
        // admins may address any bucket (admin tooling); everyone else is
        // validated against their DB-backed org membership. This blocks
        // cross-tenant reads like {user-a}.gborg content shown to
        // {user-b}@{other-org}.
        if !is_platform_admin(state, user) {
            let allowed = allowed_buckets_for(state, user);
            if !allowed.iter().any(|a| a == b) {
                warn!(
                    "drive: denied bucket override '{}' for user {:?} (not a member)",
                    b, user.email
                );
                return Err(err(StatusCode::FORBIDDEN, "Access denied for this bucket"));
            }
        }
        return Ok(b.to_string());
    }
    let default = &state.bucket_name;
    if default.is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "No bucket specified"));
    }
    Ok(user_scope::resolve_bucket_name(scope, user_id, bot_name, default))
}
/// Buckets the caller is entitled to address: their personal user- bucket,
/// the instance default bucket, and one {slug}.gborg per org they belong to
/// (via user_organizations or the signup-derived crm_contacts binding).
pub(super) fn allowed_buckets_for(state: &AppState, user: &AuthenticatedUser) -> Vec<String> {
    let uid = get_user_id(user);
    let mut allowed = vec![
        user_scope::get_user_bucket(&uid),
        state.bucket_name.clone(),
    ];
    if let Some(email) = user.email.as_deref().filter(|e| !e.is_empty()) {
        if let Ok(mut conn) = state.conn.get() {
            #[derive(diesel::QueryableByName)]
            struct SlugRow {
                #[diesel(sql_type = diesel::sql_types::Text)]
                slug: String,
                #[diesel(sql_type = diesel::sql_types::Text)]
                org_id: String,
            }
            let rows: Result<Vec<SlugRow>, _> = diesel::sql_query(
                "SELECT o.slug AS slug, o.org_id::text AS org_id FROM user_organizations uo \
                 JOIN organizations o ON o.org_id = uo.org_id \
                 JOIN users u ON u.id = uo.user_id \
                 WHERE lower(u.email) = lower($1) \
                 UNION \
                 SELECT o.slug AS slug, o.org_id::text AS org_id FROM crm_contacts c \
                 JOIN branches br ON br.id = c.branch_id \
                 JOIN organizations o ON o.org_id = br.org_id \
                 WHERE lower(c.email) = lower($1)",
            )
            .bind::<diesel::sql_types::Text, _>(email)
            .load(&mut conn);
            match rows {
                Ok(rows) => {
                    for r in rows {
                        // Org workspace bucket: `{tenant}.gborg` — the bucket
                        // every bot of that org actually resolves to. An org
                        // bot owns no standalone `{bot}.gbai` bucket, so
                        // nothing is granted on the bot name alone. The org
                        // slug may carry the legacy `-org` suffix (cristo-org),
                        // which is a DB alias and never part of the bucket name.
                        allowed.push(botbasic_core::utils::org_drive_bucket(&r.slug));
                        let bot_rows: Result<Vec<BotName>, _> = diesel::sql_query(
                            "SELECT b.name AS name FROM bots b WHERE b.org_id = $1::uuid",
                        )
                        .bind::<diesel::sql_types::Text, _>(&r.org_id)
                        .load(&mut conn);
                        if let Ok(bots) = bot_rows {
                            for b in bots {
                                // Grant each bot the bucket its row resolves
                                // to. A standalone bot keeps its own
                                // `{bot}.gbai`; an org bot resolves back to the
                                // `.gborg` bucket already granted above.
                                let bucket =
                                    botbasic_core::utils::bot_drive_location_for_name(&mut conn, &b.name)
                                        .bucket;
                                allowed.push(bucket);
                            }
                        }
                    }
                }
                Err(e) => {
                    warn!("drive: membership lookup failed for {email}: {e}");
                }
            }
        }
    }
    allowed
}
/// Effective user-scope id: admins may address another user's scope
/// (support tooling); everyone else is pinned to their own identity so a
/// client-supplied user_id cannot impersonate another tenant's prefix.
pub(super) fn effective_uid(state: &AppState, user: &AuthenticatedUser, requested: Option<&str>) -> String {
    if is_platform_admin(state, user) {
        requested
            .map(|s| s.to_string())
            .unwrap_or_else(|| get_user_id(user))
    } else {
        get_user_id(user)
    }
}
pub(super) fn get_drive(state: &AppState) -> Result<&Arc<dyn botlib::traits::DriveRepository>, (StatusCode, Json<serde_json::Value>)> {
    state
        .drive
        .as_ref()
        .ok_or_else(|| err(StatusCode::SERVICE_UNAVAILABLE, "Drive storage not configured"))
}
pub(super) fn normalize_path(path: &str) -> String {
    path.trim_matches('/').to_string()
}
pub(super) fn build_file_list_items(
    prefix: &str,
    keys: &[String],
    metadata_map: &HashMap<String, u64>,
) -> Vec<FileListItem> {
    let prefix_trimmed = prefix.trim_end_matches('/');
    let prefix_len = if prefix_trimmed.is_empty() {
        0
    } else {
        prefix_trimmed.len() + 1
    };

    let mut dirs: HashMap<String, FileListItem> = HashMap::new();
    let mut files: Vec<FileListItem> = Vec::new();

    for key in keys {
        let relative = if key.starts_with(prefix) {
            &key[prefix_len..]
        } else {
            key.as_str()
        };
        if relative.is_empty() {
            continue;
        }

        let parts: Vec<&str> = relative.splitn(2, '/').collect();
        if parts.len() == 2 {
            let dir_name = parts[0];
            dirs.entry(dir_name.to_string()).or_insert_with(|| FileListItem {
                name: dir_name.to_string(),
                path: if prefix_trimmed.is_empty() {
                    dir_name.to_string()
                } else {
                    format!("{prefix_trimmed}/{dir_name}")
                },
                is_dir: true,
                size: 0,
                modified: None,
                is_kb: false,
                is_public: false,
            });
        } else {
            let file_name = parts[0];
            let is_kb = key.contains(".gbkb");
            let size = metadata_map.get(key).copied().unwrap_or(0);
            let modified = None;
            files.push(FileListItem {
                name: file_name.to_string(),
                path: if prefix_trimmed.is_empty() {
                    file_name.to_string()
                } else {
                    format!("{prefix_trimmed}/{file_name}")
                },
                is_dir: false,
                size,
                modified,
                is_kb,
                is_public: false,
            });
        }
    }

    let mut items: Vec<FileListItem> = dirs.into_values().collect();
    items.sort_by(|a, b| a.name.cmp(&b.name));
    files.sort_by(|a, b| a.name.cmp(&b.name));
    items.extend(files);
    items
}
pub(super) fn get_user_id(user: &AuthenticatedUser) -> String {
    if let Some(ref email) = user.email {
        if !email.is_empty() && email != "session-user" {
            return email.clone();
        }
    }
    if user.user_id.is_nil() { "default".to_string() } else { user.user_id.to_string() }
}
pub(super) fn is_admin_user(user: &AuthenticatedUser) -> bool {
    user.is_admin() || user.is_super_admin()
}
/// Drive tenant checks (#1401, #1387): the cross-tenant bypass is reserved
/// for PLATFORM admins — an explicit `superadmin` role or a member of the
/// reserved root org (`organizations.slug = 'default'`, the first/default
/// account that owns the seeded cloud catalog). Plain org "admin" role stays
/// confined to its own tenant: RBAC's global `admin` group is also how ORG
/// admins are modeled, so treating them as platform admins let e.g.
/// contato@sentient read pragmatismo's buckets.
pub(super) fn is_platform_admin(state: &AppState, user: &AuthenticatedUser) -> bool {
    if user.is_super_admin() {
        return true;
    }
    let email = match user.email.as_deref().filter(|e| !e.is_empty()) {
        Some(e) => e,
        None => return false,
    };
    let mut conn = match state.conn.get() {
        Ok(c) => c,
        Err(e) => {
            warn!("drive: platform-admin check skipped (DB pool error): {e}");
            return false;
        }
    };
    #[derive(diesel::QueryableByName)]
    struct CountRow {
        #[diesel(sql_type = BigInt)]
        count: i64,
    }
    diesel::sql_query(
        "SELECT COUNT(*) AS count FROM user_organizations uo \
         JOIN organizations o ON o.org_id = uo.org_id \
         JOIN users u ON u.id = uo.user_id \
         WHERE lower(u.email) = lower($1) AND o.slug = 'default'",
    )
    .bind::<Text, _>(email)
    .get_result::<CountRow>(&mut conn)
    .map(|r| r.count > 0)
    .unwrap_or(false)
}
