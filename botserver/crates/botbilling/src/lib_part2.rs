use super::*;
use diesel::prelude::*;

pub fn default_product_config() -> ProductConfig {
    let mut plans = HashMap::new();

    plans.insert("free".to_string(), PlanConfig {
        name: "Free".to_string(),
        description: Some("Um bot, 20MB, apps ilimitados — gratuito".to_string()),
        price: PlanPrice::Free,
        limits: PlanLimits {
            messages_per_day: LimitValue::Limited(10),
            storage_mb: LimitValue::Limited(20),
            bots: LimitValue::Limited(1),
            users: LimitValue::Limited(1),
            api_calls_per_day: LimitValue::Limited(100),
            signups_per_day: Some(LimitValue::Limited(1)),
            kb_documents: LimitValue::Limited(10),
            apps: LimitValue::Unlimited,
            emails_per_day: LimitValue::Limited(10),
            builds_per_day: LimitValue::Limited(5),
            uploads_per_hour: LimitValue::Limited(5),
        },
        features: vec![
            "basic_chat".to_string(),
            "file_upload".to_string(),
            "email_support".to_string(),
        ],
        stripe_price_id: None,
        trial_days: None,
    });

    plans.insert("shared".to_string(), PlanConfig {
        name: "Shared".to_string(),
        description: Some("Seu escritório digital com 50GB, BYOK e 5 workspaces".to_string()),
        price: PlanPrice::Fixed {
            amount: 399,
            currency: "usd".to_string(),
            period: BillingPeriod::Monthly,
        },
        limits: PlanLimits {
            messages_per_day: LimitValue::Unlimited,
            storage_mb: LimitValue::Limited(51200),
            bots: LimitValue::Limited(5),
            users: LimitValue::Limited(5),
            api_calls_per_day: LimitValue::Unlimited,
            signups_per_day: None,
            kb_documents: LimitValue::Limited(100),
            apps: LimitValue::Limited(5),
            emails_per_day: LimitValue::Limited(500),
            builds_per_day: LimitValue::Limited(50),
            uploads_per_hour: LimitValue::Limited(50),
        },
        features: vec![
            "basic_chat".to_string(),
            "file_upload".to_string(),
            "byok".to_string(),
            "store_domains".to_string(),
            "store_phones".to_string(),
            "store_storage".to_string(),
        ],
        stripe_price_id: None,
        trial_days: Some(14),
    });

    plans.insert("private-cloud".to_string(), PlanConfig {
        name: "Private Cloud".to_string(),
        description: Some("Sua nuvem privada com VPS, ilimitado e marca própria".to_string()),
        price: PlanPrice::Custom,
        limits: PlanLimits {
            messages_per_day: LimitValue::Unlimited,
            storage_mb: LimitValue::Unlimited,
            bots: LimitValue::Unlimited,
            users: LimitValue::Unlimited,
            api_calls_per_day: LimitValue::Unlimited,
            signups_per_day: None,
            kb_documents: LimitValue::Unlimited,
            apps: LimitValue::Unlimited,
            emails_per_day: LimitValue::Unlimited,
            builds_per_day: LimitValue::Unlimited,
            uploads_per_hour: LimitValue::Unlimited,
        },
        features: vec![
            "basic_chat".to_string(),
            "file_upload".to_string(),
            "byok".to_string(),
            "store_domains".to_string(),
            "store_phones".to_string(),
            "store_storage".to_string(),
            "store_vps".to_string(),
            "store_gpu".to_string(),
            "custom_branding".to_string(),
            "audit_logs".to_string(),
            "api_access".to_string(),
            "analytics".to_string(),
            "sso_saml".to_string(),
            "sla_guarantee".to_string(),
            "dedicated_support".to_string(),
            "on_premise".to_string(),
        ],
        stripe_price_id: None,
        trial_days: None,
    });



    ProductConfig {
        branding: BrandingConfig {
            name: "General Bots".to_string(),
            logo: None,
            primary_color: "#1976d2".to_string(),
            secondary_color: None,
            favicon: None,
        },
        plans,
        features: vec![
            "basic_chat".to_string(),
            "file_upload".to_string(),
            "byok".to_string(),
            "store_domains".to_string(),
            "store_phones".to_string(),
            "store_storage".to_string(),
            "store_vps".to_string(),
            "store_gpu".to_string(),
            "email_support".to_string(),
            "priority_support".to_string(),
            "custom_branding".to_string(),
            "api_access".to_string(),
            "analytics".to_string(),
            "sso_saml".to_string(),
            "sla_guarantee".to_string(),
            "dedicated_support".to_string(),
            "on_premise".to_string(),
            "audit_logs".to_string(),
        ],
        stripe: None,
    }
}
pub type GetDefaultBotFn = fn(&mut diesel::PgConnection) -> Uuid;
/// Queries the database for the first active bot's branch_id, falling back to Uuid::nil() if none exists.
pub fn query_first_bot(conn: &mut diesel::PgConnection) -> Uuid {
    use diesel::prelude::*;
    use diesel::sql_types::Uuid as SqlUuid;

    #[derive(diesel::QueryableByName, Debug)]
    struct BotRow {
        #[diesel(sql_type = SqlUuid)]
        branch_id: Uuid,
    }

    let result = diesel::sql_query(
        "SELECT branch_id FROM bots WHERE is_active = true ORDER BY created_at LIMIT 1"
    )
    .load::<BotRow>(conn);
    match result {
        Ok(rows) if !rows.is_empty() => rows[0].branch_id,
        _ => Uuid::nil(),
    }
}
pub fn get_bot_context(pool: &DbPool, get_default_bot: &Option<GetDefaultBotFn>) -> Uuid {
    let Ok(mut conn) = pool.get() else {
        return Uuid::nil();
    };
    resolve_branch_id(&mut conn, get_default_bot)
}
/// `(org_id, bot_id)` for a branch.
///
/// The `billing_*` tables carry both columns NOT NULL (`6.0.9-01-billing`), but
/// the crate never wrote them: the local declarations were missing the two
/// columns (`#1517`). Every invoice/payment/quote insert resolves them here,
/// mirroring `botproducts::org_for_branch` / `bot_for_branch` and returning the
/// nil UUID when the branch cannot be resolved.
pub(crate) fn scope_for_branch(conn: &mut diesel::PgConnection, branch_id: Uuid) -> (Uuid, Uuid) {
    #[derive(diesel::QueryableByName)]
    struct OrgRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        org_id: Uuid,
    }
    #[derive(diesel::QueryableByName)]
    struct BotRow {
        #[diesel(sql_type = diesel::sql_types::Uuid)]
        id: Uuid,
    }
    let org = diesel::sql_query("SELECT org_id FROM branches WHERE id = $1 LIMIT 1")
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .get_result::<OrgRow>(conn)
        .map(|r| r.org_id)
        .unwrap_or_else(|_| Uuid::nil());
    let bot = diesel::sql_query(
        "SELECT id FROM bots WHERE branch_id = $1 \
         ORDER BY is_default_for_branch DESC, created_at ASC LIMIT 1",
    )
    .bind::<diesel::sql_types::Uuid, _>(branch_id)
    .get_result::<BotRow>(conn)
    .map(|r| r.id)
    .unwrap_or_else(|_| Uuid::nil());
    (org, bot)
}

pub(crate) fn resolve_branch_id(
    conn: &mut diesel::PgConnection,
    get_default_bot: &Option<GetDefaultBotFn>,
) -> Uuid {
    match get_default_bot {
        Some(f) => {
            let bid = f(conn);
            if bid == Uuid::nil() {
                Uuid::nil()
            } else {
                bid
            }
        }
        None => Uuid::nil(),
    }
}
