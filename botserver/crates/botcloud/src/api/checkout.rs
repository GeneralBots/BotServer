use super::*;


/// `GET /api/cloud/admin/server-capacity`
///
/// Returns real-time server capacity metrics for SaaS admin dashboard.
pub(crate) async fn get_server_capacity(
    State(_service): State<Arc<SaasService>>,
) -> Json<serde_json::Value> {
    let capacity = botbilling::server_capacity::calculate_server_capacity(
        &botbilling::server_capacity::ServerCapacityConfig::default(),
        0, 0,
    );
    Json(serde_json::json!({
        "server": {
            "cpu_cores": capacity.cpu_cores,
            "cpu_usage_pct": (capacity.cpu_usage_pct * 100.0).round() / 100.0,
            "ram_total_gb": (capacity.ram_total_gb * 100.0).round() / 100.0,
            "ram_used_gb": (capacity.ram_used_gb * 100.0).round() / 100.0,
            "ram_available_gb": (capacity.ram_available_gb * 100.0).round() / 100.0,
        },
        "saas_capacity": {
            "available_free_slots": capacity.available_free_slots,
            "available_shared_slots": capacity.available_shared_slots,
            "new_signups_allowed": capacity.new_signups_allowed,
            "capacity_health": capacity.capacity_health,
            "pressure_index": (capacity.pressure_index * 100.0).round() / 100.0,
        },
    }))
}

/// `POST /api/cloud/checkout`
///
/// Creates invoice in billing + contact/deal in CRM + Stripe session.
pub(crate) async fn handle_checkout(
    State(service): State<Arc<SaasService>>,
    Json(body): Json<CheckoutBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let payload: CalculatorPayload = serde_json::from_str(&body.payload)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid payload: {e}")))?;

    let billing = &service.billing_state;
    let mut conn = billing.pool.get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

    let branch_id = botbilling::get_bot_context(&billing.pool, &billing.get_default_bot);
    let now = chrono::Utc::now();

    let effective_branch_id = if branch_id == Uuid::nil() {
        Uuid::nil()
    } else {
        branch_id
    };

    let customer_email = body.email.clone();
    let customer_name = body.organization_name.clone()
        .unwrap_or_else(|| format!("{} Customer", payload.plan));

    let total_cents = (payload.total * 100.0) as u64;
    let total_value = total_cents as f64;
    let invoice_id = Uuid::new_v4();
    let invoice_number = botbilling::api_models::generate_invoice_number(&mut conn, effective_branch_id);

    let (org_id, bot_id) = botbilling::scope_for_branch(&mut conn, effective_branch_id);

    let invoice = botbilling::api_models::BillingInvoice {
        id: invoice_id, branch_id: effective_branch_id, invoice_number,
        customer_id: None,
        customer_name: customer_name.clone(),
        customer_email: Some(customer_email.clone()),
        customer_address: None, status: "draft".to_string(),
        issue_date: now.date_naive(),
        due_date: (now + chrono::Duration::days(30)).date_naive(),
        subtotal: botbilling::api_models::bd(total_value),
        tax_rate: botbilling::api_models::bd(0.0),
        tax_amount: botbilling::api_models::bd(0.0),
        discount_percent: botbilling::api_models::bd(0.0),
        discount_amount: botbilling::api_models::bd(0.0),
        total: botbilling::api_models::bd(total_value),
        amount_paid: botbilling::api_models::bd(0.0),
        amount_due: botbilling::api_models::bd(total_value),
        currency: payload.currency.clone(),
        notes: Some(format!("SaaS: plan={}, period={}, storage={}GB", payload.plan, payload.period, payload.storage)),
        terms: None, footer: None, paid_at: None, sent_at: None, voided_at: None,
        created_at: now, updated_at: now,
        org_id, bot_id,
    };

    diesel::insert_into(botbilling::schema::billing_invoices::table)
        .values(&invoice)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert: {e}")))?;

    let line_item = botbilling::api_models::BillingInvoiceItem {
        id: Uuid::new_v4(), invoice_id,
        product_id: None,
        description: format!("{} - {} ({}GB, AI: {:?})", payload.plan, payload.period, payload.storage, payload.ai),
        quantity: botbilling::api_models::bd(1.0),
        unit_price: botbilling::api_models::bd(total_value),
        discount_percent: botbilling::api_models::bd(0.0),
        tax_rate: botbilling::api_models::bd(0.0),
        amount: botbilling::api_models::bd(total_value),
        sort_order: 0, created_at: now,
    };

    diesel::insert_into(botbilling::schema::billing_invoice_items::table)
        .values(&line_item)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert item: {e}")))?;

    // --- Notification: invoice generated ---
    let mut vars = notifier::EmailVars::new(
        &customer_name, &customer_email, &payload.plan, total_value, &payload.currency,
    );
    vars.invoice_id = invoice_id.to_string();
    notifier::notify_invoice_created(&vars);

    // --- CRM integration: creates contact and deal ---
    let contact_id = integration::create_crm_contact(
        service.pool(),
        effective_branch_id,
        &customer_name,
        &customer_email,
        None,  // checkout: no password set
    )
    .map_err(|e| {
        tracing::warn!("CRM contact creation failed (non-fatal): {e}");
        Uuid::nil()
    });

    let _deal_id = integration::create_crm_deal(
        service.pool(),
        effective_branch_id,
        contact_id.unwrap_or(Uuid::nil()),
        invoice_id,
        &format!("Assinatura {} - {}", payload.plan, customer_name),
        total_value,
        &payload.currency,
    )
    .map_err(|e| {
        tracing::warn!("CRM deal creation failed (non-fatal): {e}");
        Uuid::nil()
    });

    // --- Stripe ---
    let stripe_customer = service.stripe
        .create_customer(botbilling::stripe_integration::CreateCustomerParams {
            email: customer_email.clone(),
            name: Some(customer_name.clone()),
            organization_id: invoice_id,
            metadata: std::collections::HashMap::new(),
        })
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe customer: {e}")))?;

    // Persist the Stripe Customer mapping so SetupIntent card management and
    // webhook events can resolve the owning branch later.
    crate::payment_cards::persist_customer_mapping(
        &service,
        effective_branch_id,
        &stripe_customer.id,
        &customer_email,
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe customer mapping: {e}")))?;

    let plan_config = botbilling::default_product_config();
    let plan = plan_config.plans.get(&payload.plan)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, format!("Plan '{}' not found", payload.plan)))?;

    let cancel_url = format!("{}/cloud/checkout/cancel", service.config.base_url);

    let session = service.stripe
        .create_checkout_session(
            botbilling::stripe_integration::CreateCheckoutSessionParams {
                customer_id: stripe_customer.id,
                price_id: payload.plan.clone(),
                success_url: format!("{}/cloud/checkout/success?session_id={{CHECKOUT_SESSION_ID}}&invoice={}", service.config.base_url, invoice_id),
                cancel_url,
                trial_days: plan.trial_days,
                metadata: std::collections::HashMap::from([
                    ("invoice_id".to_string(), invoice_id.to_string()),
                    ("plan".to_string(), payload.plan.clone()),
                    ("account_email".to_string(), customer_email.clone()),
                ]),
            },
        )
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe session: {e}")))?;

    Ok(Json(serde_json::json!({
        "checkout_url": session.url,
        "session_id": session.id,
        "invoice_id": invoice_id,
        "status": "redirecting",
    })))
}

/// `GET /api/cloud/checkout/success`
///
/// Confirma pagamento Stripe, atualiza fatura, cria GL entry + subscription.
pub(crate) async fn checkout_success(
    State(service): State<Arc<SaasService>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let session_id = params.get("session_id")
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing session_id".to_string()))?;
    let invoice_id = params.get("invoice")
        .and_then(|v| Uuid::parse_str(v).ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "missing invoice".to_string()))?;

    let session = service.stripe
        .retrieve_checkout_session(session_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Stripe: {e}")))?;

    let is_complete = session.status == "complete";
    let now = chrono::Utc::now();

    {
        let mut conn = service.pool().get()
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;

        diesel::update(
            botbilling::schema::billing_invoices::table
                .filter(botbilling::schema::billing_invoices::id.eq(invoice_id))
        )
        .set((
            botbilling::schema::billing_invoices::status.eq(
                if is_complete { "paid" } else { "pending" }
            ),
            botbilling::schema::billing_invoices::paid_at.eq(Some(now)),
            botbilling::schema::billing_invoices::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update: {e}")))?;
    }

    // --- Post-payment integrations ---
    if is_complete {
        let invoice = {
            let mut conn = service.pool().get()
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB: {e}")))?;
            botbilling::schema::billing_invoices::table
                .filter(botbilling::schema::billing_invoices::id.eq(invoice_id))
                .first::<botbilling::api_models::BillingInvoice>(&mut conn)
                .map_err(|_| (StatusCode::NOT_FOUND, "Invoice not found".to_string()))?
        };

        let total = botbilling::api_models::bd_to_f64(&invoice.total);

        // 1. CRM: marca deal como ganho
        let deal_name = format!("Assinatura - {}", invoice.customer_name.as_str());
        let _ = integration::win_crm_deal(
            service.pool(),
            invoice.branch_id,
            &deal_name,
        ).map_err(|e| tracing::warn!("CRM win deal failed: {e}"));

        // 2. ERP (GL): posts accounting entry
        let _ = integration::create_gl_entry_for_invoice(
            service.pool(),
            invoice_id,
            total,
            invoice.customer_name.as_str(),
        ).map_err(|e| tracing::warn!("GL entry creation failed: {e}"));

        // 3. Subscription: cria registro de assinatura recorrente
        let plan_label = session
            .metadata
            .as_ref()
            .and_then(|m| m.get("plan"))
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let _ = integration::create_billing_subscription(integration::NewSubscription {
            pool: service.pool(),
            branch_id: invoice.branch_id,
            customer_name: invoice.customer_name.as_str(),
            customer_email: invoice.customer_email.as_deref().unwrap_or(""),
            plan: &plan_label,
            total,
            currency: invoice.currency.as_str(),
            invoice_id,
            period: "monthly",
        })
        .map_err(|e| tracing::warn!("Subscription creation failed: {e}"));

        // --- Post-payment notifications ---
        let mut vars = notifier::EmailVars::new(
            invoice.customer_name.as_str(),
            invoice.customer_email.as_deref().unwrap_or(""),
            &plan_label,
            total,
            invoice.currency.as_str(),
        );
        vars.invoice_id = invoice_id.to_string();
        notifier::notify_payment_success(&vars);
        notifier::notify_subscription_activated(&vars);
    }

    Ok(Json(serde_json::json!({
        "status": if is_complete { "completed" } else { "pending" },
        "customer": session.customer,
        "subscription": session.subscription,
    })))
}

/// `GET /api/cloud/plans`
pub(crate) async fn list_plans(
    State(_service): State<Arc<SaasService>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = botbilling::default_product_config();
    let mut plans = serde_json::Map::new();

    for (id, plan) in &config.plans {
        let price = match &plan.price {
            botbilling::PlanPrice::Free => serde_json::json!({"type": "free"}),
            botbilling::PlanPrice::Fixed { amount, currency, period } => serde_json::json!({
                "type": "fixed", "amount": amount, "currency": currency, "period": period,
            }),
            botbilling::PlanPrice::Custom => serde_json::json!({"type": "custom"}),
        };

        plans.insert(id.clone(), serde_json::json!({
            "name": plan.name, "description": plan.description,
            "price": price, "features": plan.features,
            "trial_days": plan.trial_days,
            "limits": {
                "messages_per_day": plan.limits.messages_per_day.value(),
                "storage_mb": plan.limits.storage_mb.value(),
                "bots": plan.limits.bots.value(),
                "users": plan.limits.users.value(),
            },
        }));
    }

    Ok(Json(serde_json::json!({ "branding": config.branding, "plans": plans })))
}

/// `GET /api/cloud/plans/{plan_id}`
pub(crate) async fn get_plan_detail(
    State(_service): State<Arc<SaasService>>,
    axum::extract::Path(plan_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let config = botbilling::default_product_config();
    let plan = config.plans.get(&plan_id)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Plan '{}' not found", plan_id)))?;

    let price = match &plan.price {
        botbilling::PlanPrice::Free => serde_json::json!({"type": "free"}),
        botbilling::PlanPrice::Fixed { amount, currency, period } => serde_json::json!({
            "type": "fixed", "amount": amount, "currency": currency, "period": period,
        }),
        botbilling::PlanPrice::Custom => serde_json::json!({"type": "custom"}),
    };

    Ok(Json(serde_json::json!({
        "id": plan_id,
        "name": plan.name,
        "description": plan.description,
        "price": price,
        "features": plan.features,
        "trial_days": plan.trial_days,
        "limits": {
            "messages_per_day": plan.limits.messages_per_day.value(),
            "storage_mb": plan.limits.storage_mb.value(),
            "bots": plan.limits.bots.value(),
            "users": plan.limits.users.value(),
            "api_calls_per_day": plan.limits.api_calls_per_day.value(),
            "kb_documents": plan.limits.kb_documents.value(),
            "apps": plan.limits.apps.value(),
        },
    })))
}

// ─────────────────────────────────────────────────────────────────────────────
// Auth: Login (JWT stub — real auth via Zitadel/OIDC in production)
// ─────────────────────────────────────────────────────────────────────────────

/// `POST /api/cloud/auth/login`
///
/// Validates credentials and returns a JWT token for the management portal.
/// #1262 — `resolve_directory_service_token` (boot) only accepts candidates
/// from `directory_config.json` and `conf/directory/admin-pat.txt`. When the
/// config file loses its `service_token` key (observed on prod), login
/// silently skipped the Zitadel password check and every correct password
/// was rejected with "Invalid credentials" and no log line. This fallback
/// reads the canonical Vault secret (`gbo/directory`, `service_token`) so
/// password verification survives a config regression.
pub(crate) fn directory_token_from_vault() -> Option<String> {
    let sm = botcoresecrets::manager::SecretsManager::get_clone().ok()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build();
        let result = if let Ok(rt) = rt {
            rt.block_on(async move {
                sm.get_secret(botcoresecrets::paths::SecretPaths::DIRECTORY)
                    .await
                    .ok()
                    .and_then(|s| s.get("service_token").cloned())
                    .filter(|t| !t.is_empty())
            })
        } else {
            None
        };
        let _ = tx.send(result);
    });
    rx.recv_timeout(std::time::Duration::from_secs(5))
        .ok()
        .flatten()
}
