use super::*;

/// `POST /api/cloud/auth/signup`
///
/// Creates organization in DB + contact in CRM, returning the IDs.
/// Same trust model as `botcoredirectory::client::is_private_host`: an
/// internal deployment talks to the directory service over plain http on a
/// private network (container-to-container, e.g. http://10.0.0.x). Signup
/// must not skip identity creation in that topology — it did once and left
/// new organizations with logins that could never succeed. Only passwords to
/// PUBLIC hosts over plain http are refused.
pub(crate) async fn handle_signup(
    State(service): State<Arc<SaasService>>,
    Json(body): Json<SignupBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let bot_name = body.bot_name.as_deref()
        .map(|n| n.trim().to_lowercase().replace(' ', "-"))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| {
            body.email.split('@').next().unwrap_or("default").to_lowercase()
        });

    // Reject passwords the directory will refuse BEFORE anything is persisted:
    // the old flow created the whole org/bot transaction, then swallowed a
    // failing password set with a warn — the account existed and could never
    // log in (marcelbeiner@gmail.com was exactly that, fixed on 2026-09-29).
    // Zitadel's default complexity is: 8+ chars, upper, lower, digit, symbol.
    if let Some(password) = body.password.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        if let Some(violation) = password_policy_violation(password) {
            return Err((StatusCode::BAD_REQUEST, serde_json::json!({
                "error": "invalid_password",
                "message": violation,
            }).to_string()));
        }
    }

    // 7. Determine plan from body (default: free) — checked early for capacity gate
    let chosen_plan = body.plan.as_deref()
        .map(|p| p.to_lowercase())
        .filter(|p| p == "free" || p == "shared" || p == "private-cloud")
        .unwrap_or_else(|| "free".to_string());

    // Auto-pause free signups when server is under pressure
    // Can be disabled via SAAS_DISABLE_CAPACITY_CHECK=1 for dev/testing
    if (chosen_plan == "free" || chosen_plan == "shared")
        && std::env::var("SAAS_DISABLE_CAPACITY_CHECK").as_deref() != Ok("1")
    {
        let capacity = botbilling::server_capacity::calculate_server_capacity(
            &botbilling::server_capacity::ServerCapacityConfig::default(),
            0, 0,
        );
        tracing::info!(
            "signup capacity gate: cpu={:.1} ram={:.1} disk={:.1} (total={:.1}GB used={:.1}GB) allowed={}",
            capacity.cpu_usage_pct,
            capacity.ram_used_gb / capacity.ram_total_gb * 100.0,
            capacity.disk_used_gb / capacity.disk_total_gb.max(0.001) * 100.0,
            capacity.disk_total_gb,
            capacity.disk_used_gb,
            capacity.new_signups_allowed
        );
        if !capacity.new_signups_allowed {
            // Name the metric that tripped the gate: a silent 503 left the
            // operator guessing which resource was over its threshold (#1343).
            let cpu_pct = capacity.cpu_usage_pct;
            let ram_pct = if capacity.ram_total_gb > 0.0 {
                capacity.ram_used_gb / capacity.ram_total_gb * 100.0
            } else {
                0.0
            };
            let disk_pct = if capacity.disk_total_gb > 0.0 {
                capacity.disk_used_gb / capacity.disk_total_gb * 100.0
            } else {
                0.0
            };
            tracing::warn!(
                "signup blocked: server at capacity (cpu={cpu_pct:.1}% ram={ram_pct:.1}% disk={disk_pct:.1}%) plan={chosen_plan}"
            );
            return Err((StatusCode::SERVICE_UNAVAILABLE, format!(
                "{{ \"error\": \"server_at_capacity\", \"message\": \"{chosen_plan} plan temporarily unavailable (cpu {cpu_pct:.0}%, memory {ram_pct:.0}%, disk {disk_pct:.0}%). Please try again later.\", \"capacity_health\": \"{health}\", \"retry_after_seconds\": 300 }}",
                health = capacity.capacity_health
            )));
        }
    }

    // Determine plan config (no DB needed — done before transaction)
    let product_config = botbilling::default_product_config();
    let plan_config = product_config.plans.get(&chosen_plan)
        .ok_or((StatusCode::BAD_REQUEST, "Invalid plan".to_string()))?;

    let is_custom_plan = matches!(plan_config.price, botbilling::PlanPrice::Custom);
    let is_free_plan = matches!(plan_config.price, botbilling::PlanPrice::Free);
    let trial_days = plan_config.trial_days.unwrap_or(0);

    // Get a single DB connection for the entire signup transaction (raw SQL tx)
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB pool: {e}")))?;

    use diesel::sql_query;
    sql_query("BEGIN").execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("BEGIN: {e}")))?;

    let tx_result = (|| -> Result<(Uuid, Uuid, Uuid, String, Uuid, Option<Uuid>), String> {
        // 1. Get or create tenant
        let tenant_id = integration::get_or_create_default_tenant_inner(&mut conn)?;

        // 2. Create organization named after the bot
        let org_domain = format!("{bot_name}.org.pragmatismo.com.br");
        let org_id = integration::create_organization_inner(&mut conn, &bot_name, Some(&org_domain))?;
        integration::link_org_to_tenant_inner(&mut conn, org_id, tenant_id)?;

        // 3. Create branch with same name
        let branch_id = integration::create_branch_inner(&mut conn, org_id, tenant_id, &bot_name)?;

        // 4. Create bot record
        let (new_bot_id, org_slug) = integration::create_bot_inner(&mut conn, org_id, branch_id, &bot_name)?;

        // 5. Create CRM contact (password managed by Zitadel, not stored in DB)
        let contact_id = integration::create_crm_contact_inner(
            &mut conn, branch_id, new_bot_id, &body.name, &body.email, None::<&str>,
        )?;

        // 6. Create subscription
        // Note: org_id column in billing_recurring now references branches(id) per migration 9.16
        let subscription_id = if is_custom_plan {
            None
        } else if is_free_plan {
            Some(integration::create_free_subscription_inner(
                &mut conn, branch_id, new_bot_id, &body.name, &body.email,
            )?)
        } else {
            Some(integration::create_trial_subscription_inner(
                &mut conn, branch_id, new_bot_id, &body.name, &body.email,
                &chosen_plan, trial_days as i32,
            )?)
        };

        // 7. For free plan, create a $0 invoice to populate billing/ERP
        if is_free_plan {
            let now = chrono::Utc::now();
            let inv_id = Uuid::new_v4();
            let inv_number = botbilling::api_models::generate_invoice_number(&mut conn, branch_id);
            diesel::sql_query(
                r#"INSERT INTO billing_invoices
                   (id, org_id, bot_id, branch_id, invoice_number, customer_name, customer_email,
                    status, issue_date, due_date, subtotal, total, amount_due, amount_paid,
                    currency, notes, paid_at, created_at, updated_at)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, 'paid', $8, $9, 0, 0, 0, 0,
                           'usd', 'Free Plan activation', $10, $11, $11)"#,
            )
            .bind::<diesel::sql_types::Uuid, _>(inv_id)
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .bind::<diesel::sql_types::Uuid, _>(new_bot_id)
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .bind::<diesel::sql_types::Text, _>(&inv_number)
            .bind::<diesel::sql_types::Text, _>(&body.name)
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(Some(body.email.clone()))
            .bind::<diesel::sql_types::Date, _>(now.date_naive())
            .bind::<diesel::sql_types::Date, _>(now.date_naive())
            .bind::<diesel::sql_types::Nullable<diesel::sql_types::Timestamptz>, _>(Some(now))
            .bind::<diesel::sql_types::Timestamptz, _>(now)
            .execute(&mut conn)
            .map_err(|e| format!("Insert free plan invoice: {e}"))?;
        }

        // 8. Create default cloud workspace
        integration::create_cloud_workspace_inner(&mut conn, branch_id, &bot_name)?;

        Ok((org_id, branch_id, new_bot_id, org_slug, contact_id, subscription_id))
    })();

    let (org_id, branch_id, new_bot_id, org_slug, contact_id, subscription_id) = match tx_result {
        Ok(result) => {
            sql_query("COMMIT").execute(&mut conn)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("COMMIT: {e}")))?;
            result
        }
        Err(e) => {
            sql_query("ROLLBACK").execute(&mut conn).ok();
            return Err((StatusCode::INTERNAL_SERVER_ERROR, e));
        }
    };

    // 8. Seed default cloud CRM products (non-fatal — outside tx)
    #[cfg(feature = "saas")]
    {
        use botproducts::seed::seed_default_products;
        seed_default_products(&mut conn, branch_id);
    }

    // 9. Create org bucket `.gborg` in MinIO with bot files inside (non-fatal — outside tx)
    if let Err(e) = integration::create_bot_bucket(
        &service.config, &org_slug, &org_slug, &bot_name, body.template.as_deref(),
    ) {
        tracing::warn!("MinIO bucket creation skipped (non-fatal): {e}");
    }

    // #1500 — Vibe bootstrap: the branch's default bot becomes a git-mode
    // Vibe project so people can vibe the base bot immediately. The hook is
    // registered by the main binary when the `vibe` feature is on; a no-hook
    // build (no vibe) skips silently. Fire-and-forget: signup must not fail
    // or slow down because of Vibe/ALM (provisioning happens lazily on first
    // Vibe open, see bootstrap.rs).
    let bot_name_for_vibe = bot_name.clone();
    if let Some(hook) = botcoresecrets::hooks::take_workspace_bootstrap_hook() {
        tokio::spawn(async move {
            match hook(branch_id, bot_name_for_vibe) {
                Ok(_) => tracing::info!(
                    "vibe bootstrap hook: default bot project ready (branch {branch_id})"
                ),
                Err(e) => tracing::warn!(
                    "vibe bootstrap hook failed for branch {branch_id} (non-fatal): {e}"
                ),
            }
        });
    }

    // 10. Create identity in directory (Zitadel) if configured (non-fatal — outside tx)
    if let (Some(dir_url), Some(dir_token)) = (&service.config.directory_api_url, &service.config.directory_service_token) {
        let parts: Vec<&str> = body.name.splitn(2, ' ').collect();
        let first_name = parts.first().unwrap_or(&"");
        let last_name = parts.get(1).unwrap_or(&"");
        // Signup passwords must never cross a PUBLIC network over plain http
        // unless the operator explicitly opted in via directory_config.json's
        // allow_insecure_http (same model as botcoredirectory). Internal http
        // (private/loopback hosts) is always allowed — prod containers reach
        // Zitadel over http://10.x.x.x and skipping creation there broke
        // logins for every new signup (#1365).
        if !directory_url_allows_password(dir_url, service.config.directory_allow_insecure_http) {
            tracing::error!(
                "Directory service URL sends passwords over plain http to a public host; skipping directory identity creation for {}",
                body.email
            );
        } else {
            let client = reqwest::Client::new();

            // _import (not AddHuman): this Zitadel build silently DROPS the
            // password on AddHuman — the user is created uninitialized with an
            // init code, and both the follow-up password set ("User is not yet
            // initialized") and #1287's v2 password path (empty hash) fail.
            // _import persists a working hash in the same call (verified on
            // prod: /v2/sessions returns 201 with the imported password) and
            // takes the same payload shape.
            let mut create_req = client
                .post(format!("{dir_url}/management/v1/users/human/_import"))
                .header("Authorization", format!("Bearer {dir_token}"))
                .json(&serde_json::json!({
                    "userName": &body.email,
                    "profile": { "firstName": first_name, "lastName": last_name, "displayName": &body.name },
                    // Zitadel v1 proto field is isEmailVerified — "isVerified" was
                    // silently dropped, leaving the user uninitialized with an init
                    // code: the follow-up password set failed with
                    // "User is not yet initialized (COMMAND-M9dse)" and the account
                    // could never log in (#1365).
                    "email": { "email": &body.email, "isEmailVerified": true },
                    "password": body.password.as_deref().unwrap_or(""),
                }));
            if let Some(host) = &service.config.directory_external_domain {
                create_req = create_req.header("Host", host);
            }
            let create_resp = create_req.send().await;

            match create_resp {
                Ok(resp) if resp.status().is_success() => {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(user_id) = data.get("userId").and_then(|v| v.as_str()) {
                            // Provision the local users row immediately so the
                            // first login resolves a stable subject (#1365).
                            provision_user_row(&mut conn, user_id, &body.email);
                            if let Some(password) = &body.password {
                                // #1287 — this Zitadel build (v4.13.1) stores an EMPTY
                                // hash when the password is set through v2
                                // /v2/users/{id}/password: it returns 200 and flips
                                // `passwordChanged`, but every later session check fails
                                // with "passwap: password does not match hash: " (empty
                                // hash) — the account can never log in. The v1
                                // management endpoint persists the hash correctly, so
                                // signup MUST use it.
                                let mut pw_req = client
                                    .post(format!("{dir_url}/management/v1/users/{user_id}/password"))
                                    .header("Authorization", format!("Bearer {dir_token}"))
                                    .json(&serde_json::json!({
                                        "password": password,
                                        "noVerification": true
                                    }));
                                if let Some(host) = &service.config.directory_external_domain {
                                    pw_req = pw_req.header("Host", host);
                                }
                                // The password set MUST succeed: a user without a
                                // working hash is an account that can never log in.
                                // The old flow swallowed this failure with a warn —
                                // signup returned 201, the person hit "Invalid
                                // credentials" forever, and nothing in the logs
                                // explained it. Now: verify with a real session
                                // check, and when it does not hold, tear the
                                // directory identity down and fail the signup with
                                // the directory's own reason.
                                let pw_result = pw_req.send().await;
                                let pw_ok = match pw_result {
                                    Ok(r) if r.status().is_success() => {
                                        let probe = client
                                            .post(format!("{dir_url}/v2/sessions"))
                                            .header("Authorization", format!("Bearer {dir_token}"))
                                            .json(&serde_json::json!({
                                                "checks": {
                                                    "user": { "loginName": body.email },
                                                    "password": { "password": password }
                                                }
                                            }))
                                            .send()
                                            .await;
                                        matches!(probe, Ok(pr) if pr.status().is_success())
                                    }
                                    Ok(r) => {
                                        let status = r.status();
                                        let body_text = r.text().await.unwrap_or_default();
                                        tracing::error!(
                                            "signup: Zitadel password set returned {status} for {}: {body_text}",
                                            body.email
                                        );
                                        false
                                    }
                                    Err(e) => {
                                        tracing::error!("signup: Zitadel password set failed for {}: {e}", body.email);
                                        false
                                    }
                                };
                                if !pw_ok {
                                    let _ = client
                                        .delete(format!(
                                            "{dir_url}/management/v1/users/{user_id}"
                                        ))
                                        .header("Authorization", format!("Bearer {dir_token}"))
                                        .send()
                                        .await;
                                    return Err((
                                        StatusCode::BAD_GATEWAY,
                                        serde_json::json!({
                                            "error": "identity_provisioning_failed",
                                            "message": "The account could not be created with a working password. No data was kept — please sign up again with a stronger password (8+ chars with upper, lower, digit and symbol)."
                                        })
                                        .to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
                Ok(resp) => {
                    let status = resp.status();
                    let body_text = resp.text().await.unwrap_or_default();
                    tracing::error!("signup: Zitadel user creation returned {status} for {}: {body_text}", body.email);
                    return Err((
                        StatusCode::BAD_GATEWAY,
                        serde_json::json!({
                            "error": "identity_provisioning_failed",
                            "message": "The identity provider refused to create the account. Please try again; if it persists, contact support."
                        })
                        .to_string(),
                    ));
                }
                Err(e) => {
                    tracing::error!("signup: Zitadel user creation failed for {}: {e}", body.email);
                    return Err((
                        StatusCode::BAD_GATEWAY,
                        serde_json::json!({
                            "error": "identity_provisioning_failed",
                            "message": "The identity provider is unreachable. Please try again in a moment."
                        })
                        .to_string(),
                    ));
                }
            }
        }
    }

    notifier::notify_welcome(&notifier::EmailVars::new(
        &body.name, &body.email, &chosen_plan, 0.0, "USD",
    ));

    let header = base64_url_encode(b"{\"alg\":\"HS256\",\"typ\":\"JWT\"}");
    let now_ts = (chrono::Utc::now() + chrono::Duration::hours(24)).timestamp();
    let payload = base64_url_encode(
        format!(
            "{{\"sub\":\"{}\",\"email\":\"{}\",\"org_id\":\"{}\",\"branch_id\":\"{}\",\"bot_id\":\"{}\",\"bucket\":\"{}.gborg\",\"exp\":{}}}",
            new_bot_id, body.email, org_id, branch_id, new_bot_id, org_slug, now_ts,
        ).as_bytes()
    );
    let token = jwt_sign(&header, &payload, service.config.jwt_secret.as_bytes())
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e))?;

    // Store the JWT in the global session cache so /api/auth/me recognizes it.
    // Persist to login_sessions so the session survives botserver restarts.
    {
        use botcoredirectory::auth_routes::{SESSION_CACHE, persist_session};
        let mut cache = SESSION_CACHE.write().await;
        let session_user = botcoredirectory::auth_routes::SessionUserData {
            user_id: new_bot_id.to_string(),
            email: body.email.clone(),
            username: bot_name.clone(),
            first_name: Some(body.name.clone()),
            last_name: None,
            display_name: Some(body.name.clone()),
            organization_id: Some(org_id.to_string()),
            roles: resolve_rbac_roles(&mut conn, &new_bot_id.to_string()),
            bucket: Some(format!("{}.gborg", org_slug)),
            created_at: chrono::Utc::now().timestamp(),
        };
        cache.insert(token.clone(), session_user.clone());
        persist_session(&token, &session_user);
    }

    Ok(Json(serde_json::json!({
        "status": "ok",
        "account": { "email": body.email, "name": body.name },
        "org_id": org_id,
        "branch_id": branch_id,
        "bot_id": new_bot_id,
        "bucket": format!("{}.gborg", org_slug),
        "contact_id": contact_id,
        "subscription_id": subscription_id,
        "plan": chosen_plan,
        "trial_days": trial_days,
        "token": token,
    })))
}
