use super::*;

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SubscriptionNotFound => write!(f, "Subscription not found"),
            Self::PlanNotFound(id) => write!(f, "Plan not found: {id}"),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {msg}"),
            Self::OfferNotFound => write!(f, "Retention offer not found"),
            Self::OfferExpired => write!(f, "Retention offer has expired"),
            Self::OfferAlreadyAccepted => write!(f, "Retention offer already accepted"),
            Self::BillingError(msg) => write!(f, "Billing error: {msg}"),
        }
    }
}
impl std::error::Error for LifecycleError {}
impl From<BillingError> for LifecycleError {
    fn from(err: BillingError) -> Self {
        Self::BillingError(err.to_string())
    }
}
pub async fn process_expiring_subscriptions(
    service: Arc<SubscriptionLifecycleService>,
) -> Vec<Uuid> {
    let now = Utc::now();
    let subscriptions = service.subscriptions.read().await;

    let expiring: Vec<Uuid> = subscriptions
        .values()
        .filter(|s| {
            s.status == SubscriptionStatus::Active
                && s.current_period_end <= now + Duration::days(3)
                && s.current_period_end > now
        })
        .map(|s| s.id)
        .collect();

    expiring
}
pub async fn process_expired_trials(service: Arc<SubscriptionLifecycleService>) -> Vec<Uuid> {
    let now = Utc::now();
    let mut subscriptions = service.subscriptions.write().await;

    let mut expired = Vec::new();

    for subscription in subscriptions.values_mut() {
        if subscription.status == SubscriptionStatus::Trialing
            && subscription.current_period_end <= now
        {
            subscription.status = SubscriptionStatus::Active;
            subscription.current_period_start = now;
            subscription.current_period_end = now + Duration::days(30);
            subscription.updated_at = now;
            expired.push(subscription.id);
        }
    }

    expired
}
/// Promotes DB-backed trial subscriptions (`billing_recurring`) whose trial
/// period has ended to `active` at the plan price and generates the first
/// paid invoice. The signup flow (botcloud `handle_signup`) writes trial rows
/// to this table, so the in-memory `SubscriptionLifecycleService` alone cannot
/// convert them - this DB routine is the source of truth for billing.
pub fn promote_expired_trials_in_db(conn: &mut diesel::PgConnection) -> Result<usize, String> {
    use diesel::sql_query;
    use diesel::sql_types::{Date, Double, Nullable, Text, Uuid as DieselUuid};

    #[derive(diesel::QueryableByName)]
    #[diesel(check_for_backend(diesel::pg::Pg))]
    struct TrialRow {
        #[diesel(sql_type = DieselUuid)]
        id: Uuid,
        #[diesel(sql_type = DieselUuid)]
        branch_id: Uuid,
        #[diesel(sql_type = DieselUuid)]
        org_id: Uuid,
        #[diesel(sql_type = DieselUuid)]
        bot_id: Uuid,
        #[diesel(sql_type = Text)]
        customer_name: String,
        #[diesel(sql_type = Text)]
        customer_email: String,
        #[diesel(sql_type = Text)]
        description: String,
    }

    let today = chrono::Utc::now().date_naive();

    use diesel::RunQueryDsl;
    let expired: Vec<TrialRow> = sql_query(
        "SELECT id, branch_id, org_id, bot_id, customer_name, customer_email, description \
         FROM billing_recurring WHERE status = 'trialing' AND next_invoice_date <= $1",
    )
    .bind::<Date, _>(today)
    .load(conn)
    .map_err(|e| format!("Query expired trials: {e}"))?;

    let plans = crate::default_product_config().plans;
    let mut promoted = 0usize;

    for row in &expired {
        // description format: "{plan} - {trial_days} Day Trial"
        let plan_name = row.description.split(" - ").next().unwrap_or("").to_lowercase();
        let amount_cents = match plans.get(&plan_name) {
            Some(PlanConfig { price: PlanPrice::Fixed { amount, .. }, .. }) => *amount,
            _ => {
                tracing::warn!(
                    "Trial promotion skipped for subscription {}: unknown price for plan '{}'",
                    row.id, plan_name
                );
                continue;
            }
        };
        let amount_dollars = amount_cents as f64 / 100.0;
        let new_invoice_date = today + chrono::Duration::days(30);

        // 1. Promote the subscription itself.
        sql_query(
            "UPDATE billing_recurring SET status = 'active', amount = $2, \
             start_date = $3, next_invoice_date = $3, end_date = $3, updated_at = NOW() \
             WHERE id = $1",
        )
        .bind::<DieselUuid, _>(row.id)
        .bind::<Double, _>(amount_dollars)
        .bind::<Date, _>(new_invoice_date)
        .execute(conn)
        .map_err(|e| format!("Update trial subscription {}: {e}", row.id))?;

        // 2. Create the first invoice for the new period
        let invoice_id = Uuid::new_v4();
        let inv_number = crate::api_models::generate_invoice_number(conn, row.branch_id);
        sql_query(
            r#"INSERT INTO billing_invoices
               (id, org_id, bot_id, branch_id, invoice_number, customer_name, customer_email,
                status, issue_date, due_date, subtotal, tax_rate, tax_amount,
                discount_percent, discount_amount, total, amount_due, amount_paid,
                currency, notes, created_at, updated_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, 'unpaid', $8, $9, $10, 0, 0, 0, 0, $10, $10, 0,
                       'USD', $11, NOW(), NOW())"#,
        )
        .bind::<DieselUuid, _>(invoice_id)
        .bind::<DieselUuid, _>(row.org_id)
        .bind::<DieselUuid, _>(row.bot_id)
        .bind::<DieselUuid, _>(row.branch_id)
        .bind::<Text, _>(&inv_number)
        .bind::<Text, _>(&row.customer_name)
        .bind::<Nullable<Text>, _>(Some(&row.customer_email))
        .bind::<Date, _>(today)
        .bind::<Date, _>(new_invoice_date)
        .bind::<Double, _>(amount_dollars)
        .bind::<Nullable<Text>, _>(Some(format!("{plan_name} plan - first invoice after trial")))
        .execute(conn)
        .map_err(|e| format!("Insert first invoice for {}: {e}", row.id))?;

        // 3. Link the invoice and bump the counter
        sql_query(
            "UPDATE billing_recurring SET last_invoice_id = $2, invoices_generated = invoices_generated + 1 \
             WHERE id = $1",
        )
        .bind::<DieselUuid, _>(row.id)
        .bind::<DieselUuid, _>(invoice_id)
        .execute(conn)
        .map_err(|e| format!("Link invoice for {}: {e}", row.id))?;

        promoted += 1;
        tracing::info!(
            "Promoted trial subscription {} to active ({plan_name}, ${amount_dollars:.2}), invoice {inv_number}",
            row.id
        );
    }

    Ok(promoted)
}
