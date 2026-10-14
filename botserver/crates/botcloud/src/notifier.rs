use lettre::{
    transport::smtp::authentication::Credentials, AsyncSmtpTransport, AsyncTransport,
    Message, Tokio1Executor,
};
use std::collections::HashMap;
use std::sync::OnceLock;

static SMTP_CONFIG: OnceLock<SmtpConfig> = OnceLock::new();

#[derive(Debug, Clone)]
struct SmtpConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
    from: String,
}

fn get_smtp_config() -> Option<&'static SmtpConfig> {
    SMTP_CONFIG.get()
}

fn try_send_email(to: &str, subject: &str, body: &str) -> Result<(), String> {
    let cfg = get_smtp_config().ok_or("SMTP not configured")?;

    let email = Message::builder()
        .from(cfg.from.parse().map_err(|e: lettre::address::AddressError| format!("Invalid from address: {e}"))?)
        .to(to.parse().map_err(|e: lettre::address::AddressError| format!("Invalid to address: {e}"))?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| format!("Failed to build email: {e}"))?;

    let creds = Credentials::new(cfg.user.clone(), cfg.password.clone());

    let mailer: AsyncSmtpTransport<Tokio1Executor> =
        AsyncSmtpTransport::<Tokio1Executor>::relay(&cfg.host)
            .map_err(|e| format!("SMTP relay error: {e}"))?
            .port(cfg.port)
            .credentials(creds)
            .build();

    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            mailer.send(email).await.map_err(|e| format!("SMTP send error: {e}"))
        })
    })?;

    Ok(())
}

pub struct EmailVars {
    pub customer_name: String,
    pub customer_email: String,
    pub plan: String,
    pub amount: f64,
    pub currency: String,
    pub invoice_id: String,
    pub org_id: String,
    pub bot_name: String,
    pub extra: HashMap<String, String>,
}

impl EmailVars {
    pub fn new(
        customer_name: &str,
        customer_email: &str,
        plan: &str,
        amount: f64,
        currency: &str,
    ) -> Self {
        Self {
            customer_name: customer_name.to_string(),
            customer_email: customer_email.to_string(),
            plan: plan.to_string(),
            amount,
            currency: currency.to_string(),
            invoice_id: String::new(),
            org_id: String::new(),
            bot_name: "default".to_string(),
            extra: HashMap::new(),
        }
    }
}

pub fn notify_welcome(vars: &EmailVars) {
    tracing::info!(
        target: "saas_notifier",
        "[EMAIL] welcome to={email} name={name} plan={plan}",
        email = vars.customer_email,
        name = vars.customer_name,
        plan = vars.plan,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = format!("Welcome to General Bots Cloud — {} Plan", vars.plan);
        let body = format!(
            "Hello {},\n\n\
             Welcome to General Bots Cloud! Your {} plan is now active.\n\n\
             You can start creating bots and managing your workspace at:\n\
             https://cloud.generalbots.com/dashboard\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, vars.plan
        );
        if let Err(e) = try_send_email(&vars.customer_email, &subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send welcome email: {e}");
        }
    }
}

pub fn notify_invoice_created(vars: &EmailVars) {
    tracing::info!(
        target: "saas_notifier",
        "[EMAIL] invoice_created to={email} invoice={inv} amount={amt} {cur}",
        email = vars.customer_email,
        inv = vars.invoice_id,
        amt = vars.amount,
        cur = vars.currency,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = format!("Invoice #{} — General Bots Cloud", vars.invoice_id);
        let body = format!(
            "Hello {},\n\n\
             A new invoice has been created:\n\
             Invoice: {}\n\
             Amount: ${:.2} {}\n\n\
             View details at: https://cloud.generalbots.com/invoices\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, vars.invoice_id, vars.amount, vars.currency
        );
        if let Err(e) = try_send_email(&vars.customer_email, &subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send invoice email: {e}");
        }
    }
}

pub fn notify_payment_success(vars: &EmailVars) {
    tracing::info!(
        target: "saas_notifier",
        "[EMAIL] payment_success to={email} plan={plan} amount={amt} {cur}",
        email = vars.customer_email,
        plan = vars.plan,
        amt = vars.amount,
        cur = vars.currency,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = "Payment Confirmed — General Bots Cloud";
        let body = format!(
            "Hello {},\n\n\
             Your payment of ${:.2} {} has been confirmed.\n\
             Plan: {}\n\n\
             Thank you for your subscription!\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, vars.amount, vars.currency, vars.plan
        );
        if let Err(e) = try_send_email(&vars.customer_email, subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send payment success email: {e}");
        }
    }
}

pub fn notify_subscription_activated(vars: &EmailVars) {
    tracing::info!(
        target: "saas_notifier",
        "[EMAIL] subscription_activated to={email} plan={plan}",
        email = vars.customer_email,
        plan = vars.plan,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = "Subscription Activated — General Bots Cloud";
        let body = format!(
            "Hello {},\n\n\
             Your {} subscription is now active.\n\n\
             You can manage your subscription at:\n\
             https://cloud.generalbots.com/services\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, vars.plan
        );
        if let Err(e) = try_send_email(&vars.customer_email, subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send activation email: {e}");
        }
    }
}

pub fn notify_recurring_charge(vars: &EmailVars) {
    tracing::info!(
        target: "saas_notifier",
        "[EMAIL] recurring_charge to={email} invoice={inv} amount={amt} {cur}",
        email = vars.customer_email,
        inv = vars.invoice_id,
        amt = vars.amount,
        cur = vars.currency,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = format!("Recurring Charge — Invoice #{}", vars.invoice_id);
        let body = format!(
            "Hello {},\n\n\
             Your recurring subscription has been charged:\n\
             Invoice: {}\n\
             Amount: ${:.2} {}\n\n\
             View details at: https://cloud.generalbots.com/invoices\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, vars.invoice_id, vars.amount, vars.currency
        );
        if let Err(e) = try_send_email(&vars.customer_email, &subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send recurring charge email: {e}");
        }
    }
}

pub fn notify_payment_failed(vars: &EmailVars) {
    tracing::warn!(
        target: "saas_notifier",
        "[EMAIL] payment_failed to={email} invoice={inv} amount={amt} {cur}",
        email = vars.customer_email,
        inv = vars.invoice_id,
        amt = vars.amount,
        cur = vars.currency,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = "Payment Failed — Action Required";
        let body = format!(
            "Hello {},\n\n\
             Your payment of ${:.2} {} has failed.\n\
             Invoice: {}\n\n\
             Please update your payment method at:\n\
             https://cloud.generalbots.com/settings\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, vars.amount, vars.currency, vars.invoice_id
        );
        if let Err(e) = try_send_email(&vars.customer_email, subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send payment failed email: {e}");
        }
    }
}

pub fn notify_subscription_cancelled(vars: &EmailVars) {
    tracing::info!(
        target: "saas_notifier",
        "[EMAIL] subscription_cancelled to={email} plan={plan}",
        email = vars.customer_email,
        plan = vars.plan,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = "Subscription Cancelled — General Bots Cloud";
        let body = format!(
            "Hello {},\n\n\
             Your {} subscription has been cancelled.\n\n\
             If you'd like to reactivate, visit:\n\
             https://cloud.generalbots.com/offers\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, vars.plan
        );
        if let Err(e) = try_send_email(&vars.customer_email, subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send cancellation email: {e}");
        }
    }
}

pub fn notify_grace_period_warning(vars: &EmailVars, days_left: u32) {
    tracing::warn!(
        target: "saas_notifier",
        "[EMAIL] grace_period_warning to={email} days_left={days}",
        email = vars.customer_email,
        days = days_left,
    );

    if let Some(_cfg) = get_smtp_config() {
        let subject = "Payment Overdue — Service Suspension Warning";
        let body = format!(
            "Hello {},\n\n\
             Your payment is overdue. Your service will be suspended in {} days.\n\n\
             Please update your payment method immediately:\n\
             https://cloud.generalbots.com/settings\n\n\
             Best regards,\n\
             General Bots Team",
            vars.customer_name, days_left
        );
        if let Err(e) = try_send_email(&vars.customer_email, subject, &body) {
            tracing::warn!(target: "saas_notifier", "Failed to send grace period warning: {e}");
        }
    }
}

pub fn init_smtp(host: &str, port: u16, user: &str, password: &str, from: &str) {
    let _ = SMTP_CONFIG.set(SmtpConfig {
        host: host.to_string(),
        port,
        user: user.to_string(),
        password: password.to_string(),
        from: from.to_string(),
    });
}
