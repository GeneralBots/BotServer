use super::*;


/// Uses Zitadel sessions API when directory is configured; falls back to
/// local argon2 hash (dev mode) when Zitadel is not available.
pub(crate) async fn handle_login(
    State(service): State<Arc<SaasService>>,
    Json(body): Json<LoginBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Minimum email validation
    if !body.email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "Invalid email".to_string()));
    }

    // Try Zitadel v2 sessions API for password verification.
    // Returns (stable Zitadel user id, password-verified flag): this Zitadel
    // build omits session factors, so a created session proves the password but
    // may not expose the user id — in that case callers resolve the stable id
    // from the users table keyed by the verified email.
    // #1262 — resolve the directory token with a Vault fallback so a missing
    // `service_token` in directory_config.json can never silently disable
    // password verification. The error below makes the unavailable case
    // diagnosable instead of surfacing as a bare "Invalid credentials".
    let effective_directory_token = match (&service.config.directory_api_url, &service.config.directory_service_token) {
        (Some(_), Some(token)) if !token.is_empty() => Some(token.clone()),
        (Some(_), _) => match directory_token_from_vault() {
            Some(t) => {
                tracing::warn!(
                    "directory_config.json has no service_token - using Vault gbo/directory fallback"
                );
                Some(t)
            }
            None => {
                tracing::error!(
                    "No directory service token available - Zitadel password verification is DISABLED for this login"
                );
                None
            }
        },
        _ => None,
    };
    // Set when the directory could not be reached at all (timeout, refused
    // connection, DNS). A password that was never actually checked must not be
    // reported as a wrong password: prod saw a login return "Invalid
    // credentials" purely because the Zitadel round trip exceeded the client
    // timeout, which sends users chasing password resets that are not needed.
    let mut directory_unreachable = false;
    let (zitadel_user_id, zitadel_password_verified) = match (&service.config.directory_api_url, &effective_directory_token) {
        (Some(dir_url), Some(dir_token)) => {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(15))
                .build().ok();
            match client {
                Some(c) => {
                    // Zitadel matches `loginName` against a user's login names
                    // (username + org-domain forms). For accounts created with a
                    // bare username, sending the full email fails; try the email
                    // first, then fall back to the username prefix.
                    let username = body.email.split('@').next().unwrap_or(&body.email).to_string();
                    let login_names = [body.email.clone(), username];

                    let mut user_id: Option<String> = None;
                    let mut verified = false;

                    for login_name in &login_names {
                        // Use the v2 `checks` wrapper so the password is actually
                        // verified. A bare `{loginName, password}` flat shape makes
                        // Zitadel start a session WITHOUT checking credentials, so
                        // ANY password would mint a valid JWT. The checks wrapper
                        // runs the user + password checks and only returns a
                        // session when they pass (wrong password -> COMMAND-3M0fs).
                        let mut rb = c.post(format!("{dir_url}/v2/sessions"))
                            .header("Authorization", format!("Bearer {dir_token}"))
                            .json(&serde_json::json!({
                                "checks": {
                                    "user": { "loginName": login_name },
                                    "password": { "password": body.password }
                                }
                            }));
                        if let Some(host) = &service.config.directory_external_domain {
                            rb = rb.header("Host", host);
                        }
                        match rb.send().await {
                            Ok(r) if r.status().is_success() => {
                                verified = true;
                                // On a successful checks wrapper, the response carries
                                // the verified user factor directly; use it as the
                                // stable JWT subject (RBAC derives UUIDv5 from it).
                                let session = r.json::<serde_json::Value>().await.ok();
                                // #1263 — never fall back to `sessionId` here: it
                                // changes on every login, so the JWT sub (and
                                // everything derived from it — RBAC UUIDv5, org
                                // membership, project ownership) would rotate per
                                // session. When this build omits session factors,
                                // leave user_id None; the verified-password path
                                // below resolves the STABLE subject from the users
                                // table keyed by the verified email.
                                user_id = session.as_ref()
                                    .and_then(|v| v.get("factors").cloned())
                                    .and_then(|f| f.get("user").cloned())
                                    .and_then(|u| u.get("userId").or_else(|| u.get("id")).cloned())
                                    .and_then(|uid| uid.as_str().map(|s| s.to_string()));
                                break;
                            }
                            Ok(_) => {
                                tracing::warn!("Zitadel session check rejected credentials for loginName '{}'", login_name);
                            }
                            Err(e) => {
                                directory_unreachable = true;
                                tracing::warn!("Zitadel session check failed for loginName '{}': {}", login_name, e);
                            }
                        }
                    }

                    (user_id, verified)
                }
                None => (None, false),
            }
        }
        _ => (None, false),
    };

    // The password was never checked because the directory did not answer.
    // Say so instead of claiming the credentials are wrong.
    if directory_unreachable && !zitadel_password_verified {
        tracing::error!(
            "login for {} could not be verified: directory service unreachable (Zitadel)",
            body.email
        );
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            serde_json::json!({
                "detail": "Directory service unavailable - the password could not be verified. Please retry."
            })
            .to_string(),
        ));
    }

    // Look up user in CRM contacts for JWT claims
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    use crate::schema_ext::crm_contacts::dsl::{crm_contacts, email, id, first_name, last_name, branch_id};
    let contact_opt = crm_contacts
        .filter(email.eq(&body.email))
        .select((id, first_name, last_name, email, branch_id))
        .first::<(Uuid, Option<String>, Option<String>, Option<String>, Uuid)>(&mut conn)
        .optional()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query: {e}")))?;

    // Generate JWT (HMAC-SHA256 with configured secret)
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() + 86400 * 7; // 7 days

    let header  = base64_url_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    // Auth gate: a token is only issued after password verification.
    // 1) Zitadel v2 session check succeeded (sessionId), or
    // 2) dev-only bootstrap admin (admin-credentials.json, present only in dev).
    // NEVER fall back to an unverified CRM contact row.
    let sub = zitadel_user_id
        .map(|zid| resolve_login_subject(&mut conn, &zid, &body.email))
        .or_else(|| {
            if zitadel_password_verified {
                // Zitadel verified the password but this build does not expose
                // the user id via session factors. Derive a stable subject from
                // the users table (keyed by the verified email) so JWT subs and
                // the derived UUIDs used by RBAC/org membership stay constant
                // across logins.
                #[derive(QueryableByName)]
                struct StableUserRow {
                    // QueryableByName resolves columns by field name — the
                    // SQL alias MUST match this field (a mismatch fails at
                    // runtime, silently, and the login 401s with no log).
                    #[diesel(sql_type = diesel::sql_types::Uuid)]
                    user_id: Uuid,
                }
                match diesel::sql_query(
                    "SELECT id AS user_id FROM users WHERE email = $1 AND is_active = true LIMIT 1",
                )
                .bind::<diesel::sql_types::Text, _>(body.email.as_str())
                .get_result::<StableUserRow>(&mut conn)
                .optional()
                {
                    Ok(Some(row)) => Some(row.user_id.to_string()),
                    _ => {
                        // Password just verified but no users row exists: this
                        // is a legacy hollow account (created before signup
                        // provisioned users rows — #1365). Provision one now,
                        // keyed by a deterministic UUIDv5 of the verified
                        // email, so future logins resolve the same subject.
                        let derived = Uuid::new_v5(
                            &Uuid::NAMESPACE_DNS,
                            format!("zitadel:{}", body.email).as_bytes(),
                        );
                        let username = body.email.split('@').next().unwrap_or(&body.email).to_string();
                        match diesel::sql_query(
                            "INSERT INTO users (id, username, email, password_hash, created_at, updated_at, is_active) \
                             VALUES ($1, $2, $3, '', NOW(), NOW(), true) \
                             ON CONFLICT (id) DO NOTHING",
                        )
                        .bind::<diesel::sql_types::Uuid, _>(derived)
                        .bind::<diesel::sql_types::Text, _>(username)
                        .bind::<diesel::sql_types::Text, _>(body.email.as_str())
                        .execute(&mut conn)
                        {
                            Ok(_) => {
                                tracing::info!("Provisioned users row {derived} for verified legacy login {}", body.email);
                                Some(derived.to_string())
                            }
                            Err(e) => {
                                tracing::warn!("users row provisioning failed for {}: {e}", body.email);
                                None
                            }
                        }
                    }
                }
            } else {
                None
            }
        })
        .or_else(|| lookup_admin_credentials_user_id(&body.email, &body.password))
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                serde_json::json!({ "detail": "Invalid credentials" }).to_string(),
            )
        })?;

    // Mint verified tenant scope claims (issue #736): the branch comes from
    // the CRM contact owned by the authenticated identity — never from the
    // client. When the user has no CRM contact row, fall back to their org
    // membership binding (users → user_organizations → branches) so suite
    // apps scope to the caller's own workspace (issue #808 fix: prod admin
    // without a crm_contacts row got nil-branch scoping → empty grids).
    // The org owning the branch is resolved from the branches table.
    let branch_scope: Option<Uuid> = contact_opt
        .as_ref()
        .map(|(_, _, _, _, br)| *br)
        .or_else(|| resolve_branch_from_user_binding(&mut conn, &body.email));
    let org_scope: Option<Uuid> = branch_scope.and_then(|b| {
        #[derive(diesel::QueryableByName)]
        struct OrgRow {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            org_id: Uuid,
        }
        diesel::sql_query("SELECT org_id FROM branches WHERE id = $1 LIMIT 1")
            .bind::<diesel::sql_types::Uuid, _>(b)
            .get_result::<OrgRow>(&mut conn)
            .optional()
            .ok()
            .flatten()
            .map(|r| r.org_id)
    });

    let payload_body = match (org_scope, branch_scope) {
        (Some(org_id), Some(branch)) => {
            // The caller's workspace bucket travels in the JWT so the suite
            // shell (Drive in particular) lands in the caller's own org layout
            // instead of probing garbage: `{branch_slug}.gborg` is the org
            // workspace bucket the drive monitor materializes for the branch
            // (`beiner.gborg/beiner.gbai/...`).
            #[derive(diesel::QueryableByName)]
            struct SlugRow {
                #[diesel(sql_type = diesel::sql_types::Text)]
                slug: String,
            }
            let branch_slug: Option<String> = diesel::sql_query(
                "SELECT slug FROM branches WHERE id = $1 LIMIT 1",
            )
            .bind::<diesel::sql_types::Uuid, _>(branch)
            .get_result::<SlugRow>(&mut conn)
            .optional()
            .ok()
            .flatten()
            .map(|r| r.slug);
            let bucket_claim = branch_slug
                .clone()
                .map(|slug| format!("{slug}.gborg"));
            match (&org_id, &branch, &bucket_claim) {
                (_, _, Some(bucket)) => format!(
                    "{{\"sub\":\"{}\",\"email\":\"{}\",\"exp\":{},\"org_id\":\"{}\",\"branch_id\":\"{}\",\"bucket\":\"{}\"}}",
                    sub, body.email, exp, org_id, branch, bucket
                ),
                _ => format!(
                    "{{\"sub\":\"{}\",\"email\":\"{}\",\"exp\":{},\"org_id\":\"{}\",\"branch_id\":\"{}\"}}",
                    sub, body.email, exp, org_id, branch
                ),
            }
        }
        _ => format!(
            "{{\"sub\":\"{}\",\"email\":\"{}\",\"exp\":{}}}",
            sub,
            body.email,
            exp
        ),
    };
    let payload = base64_url_encode(payload_body.as_bytes());
    let token = jwt_sign(&header, &payload, service.config.jwt_secret.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;
    tracing::info!(
        "cloud login minted JWT for {} (secret fp {:?})",
        body.email,
        &service.config.jwt_secret[..8.min(service.config.jwt_secret.len())]
    );

    let (user_name, found) = if let Some((_, fn_, ln_, _, _)) = &contact_opt {
        let n = [fn_.as_deref().unwrap_or(""), ln_.as_deref().unwrap_or("")]
            .iter().filter(|s| !s.is_empty()).cloned().collect::<Vec<_>>().join(" ");
        (if n.is_empty() { body.email.split('@').next().unwrap_or("User").to_string() } else { n }, true)
    } else {
        (body.email.split('@').next().unwrap_or("User").to_string(), false)
    };

    // Cache the session so /api/auth/me resolves the bearer token to a real
    // user (signup already stores it; login must too — otherwise the suite
    // treats freshly logged-in users as anonymous, issue #808 report).
    // Persist to login_sessions so the session survives botserver restarts
    // (same durability the suite-sso hop provides).
    // The session carries the caller's workspace bucket, so /api/auth/me can
    // hand the suite its own org layout (Drive's discoverBuckets short-circuits
    // on it — with it null the UI probed wrong buckets and hung on loading).
    let session_bucket: Option<String> = branch_scope.and_then(|b| {
        #[derive(diesel::QueryableByName)]
        struct BranchSlugRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            slug: String,
        }
        diesel::sql_query("SELECT slug FROM branches WHERE id = $1 LIMIT 1")
            .bind::<diesel::sql_types::Uuid, _>(b)
            .get_result::<BranchSlugRow>(&mut conn)
            .optional()
            .ok()
            .flatten()
            .map(|r| format!("{}.gborg", r.slug))
    });
    {
        use botcoredirectory::auth_routes::{SESSION_CACHE, persist_session};
        let mut cache = SESSION_CACHE.write().await;
        let session_user = botcoredirectory::auth_routes::SessionUserData {
            user_id: sub.clone(),
            email: body.email.clone(),
            username: body.email.split('@').next().unwrap_or("user").to_string(),
            first_name: contact_opt.as_ref().and_then(|(_, fn_, _, _, _)| fn_.clone()),
            last_name: contact_opt.as_ref().and_then(|(_, _, ln_, _, _)| ln_.clone()),
            display_name: Some(user_name.clone()),
            organization_id: org_scope.map(|o| o.to_string()),
            roles: resolve_rbac_roles(&mut conn, &sub),
            bucket: session_bucket,
            created_at: chrono::Utc::now().timestamp(),
        };
        cache.insert(token.clone(), session_user.clone());
        persist_session(&token, &session_user);
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "token": token,
        "email": body.email,
        "name": user_name,
        "is_new": !found,
    })))
}
