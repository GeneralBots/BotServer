use super::*;


/// `GET /api/cloud/llm-providers`
pub(crate) async fn list_llm_providers() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Try to read from llm_releases.json for the complete catalog
    let json_path = PathBuf::from("3rdparty/llm_releases.json");
    if let Ok(content) = tokio::fs::read_to_string(&json_path).await {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(providers) = val.get("providers") {
                return Ok(Json(serde_json::json!({ "providers": providers })));
            }
        }
    }

    // Fallback: return hardcoded providers so the frontend always works
    Ok(Json(serde_json::json!({
        "providers": [
            {
                "id": "zhipu", "name": "GLM (Zhipu AI)",
                "description": "Chinese models from Zhipu AI with excellent reasoning performance and long context.",
                "website": "https://open.bigmodel.cn", "requires_byok": false, "icon": "glm",
                "models": [
                    {"id": "glm-5.2", "name": "GLM-5.2", "context": 262144, "description": "Flagship model with deep reasoning and agentic capabilities", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools","vision"]},
                    {"id": "glm-5.2-air", "name": "GLM-5.2-Air", "context": 131072, "description": "Lightweight and fast for chatbots", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools"]},
                    {"id": "glm-4-flash", "name": "GLM-4-Flash", "context": 131072, "description": "Free tier with high request rate", "pricing": "free-tier", "capabilities": ["chat"]}
                ]
            },
            {
                "id": "alibaba", "name": "Qwen (Alibaba Cloud)",
                "description": "Alibaba's Qwen 3.6 family — latest-generation open models with breakthrough performance.",
                "website": "https://tongyi.aliyun.com", "requires_byok": false, "icon": "qwen",
                "models": [
                    {"id": "qwen-3.6-max", "name": "Qwen 3.6-Max", "context": 262144, "description": "Most powerful in the 3.6 family", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools","reasoning"]},
                    {"id": "qwen-3.6-plus", "name": "Qwen 3.6-Plus", "context": 131072, "description": "Performance-cost balance", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools"]},
                    {"id": "qwen-3.6-turbo", "name": "Qwen 3.6-Turbo", "context": 131072, "description": "Fast and economical", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat"]}
                ]
            },
            {
                "id": "deepseek", "name": "DeepSeek",
                "description": "Deep reasoning models from DeepSeek (深度求索).",
                "website": "https://platform.deepseek.com", "requires_byok": false, "icon": "deepseek",
                "models": [
                    {"id": "deepseek-v4-flash", "name": "DeepSeek V4 Flash", "context": 131072, "description": "Latest generation — fast and powerful reasoning model", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools","reasoning"]},
                    {"id": "deepseek-r1", "name": "DeepSeek-R1", "context": 65536, "description": "Reasoning with chain-of-thought", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","reasoning"]},
                    {"id": "deepseek-v3", "name": "DeepSeek-V3", "context": 65536, "description": "Previous gen — still available for cost savings", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools"]}
                ]
            },
            {
                "id": "minimax", "name": "MiniMax",
                "description": "Chinese models with up to 1M token context.",
                "website": "https://www.minimaxi.com", "requires_byok": false, "icon": "minimax",
                "models": [
                    {"id": "minimax-text-01", "name": "MiniMax-Text-01", "context": 1048576, "description": "1M token context", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools"]},
                    {"id": "minimax-abab-6.5", "name": "MiniMax-abab6.5", "context": 131072, "description": "Efficient for conversation", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat"]}
                ]
            },
            {
                "id": "yi", "name": "Yi (01.AI)",
                "description": "Models from 01.AI (Kai-Fu Lee) with multilingual performance.",
                "website": "https://www.lingyiwanwu.com", "requires_byok": false, "icon": "yi",
                "models": [
                    {"id": "yi-lightning", "name": "Yi-Lightning", "context": 131072, "description": "Flagship with advanced reasoning", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools"]},
                    {"id": "yi-lightning-fast", "name": "Yi-Lightning-Fast", "context": 32768, "description": "Optimized for low latency", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat"]}
                ]
            },
            {
                "id": "openai", "name": "OpenAI",
                "description": "Frontier models: GPT-5.5, GPT-5.4 and o-5 reasoning.",
                "website": "https://platform.openai.com", "requires_byok": false, "icon": "openai",
                "models": [
                    {"id": "gpt-5.5", "name": "GPT-5.5", "context": 1048576, "description": "Frontier multimodal intelligence — 1M context", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","vision","tools","reasoning"]},
                    {"id": "gpt-5.4", "name": "GPT-5.4", "context": 262144, "description": "Previous frontier — still excellent for production", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","vision","tools","reasoning"]},
                    {"id": "o-5", "name": "o-5", "context": 524288, "description": "Advanced reasoning with full chain-of-thought", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","reasoning"]}
                ]
            },
            {
                "id": "anthropic", "name": "Anthropic",
                "description": "Claude Fable 5 (Mythos-class), Opus 4.8, Sonnet 4.6, Haiku 4.5 — no legacy 3.x.",
                "website": "https://console.anthropic.com", "requires_byok": false, "icon": "anthropic",
                "models": [
                    {"id": "claude-fable-5", "name": "Claude Fable 5", "context": 1048576, "description": "Mythos-class — Anthropic's most capable model", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools","vision","reasoning"]},
                    {"id": "claude-opus-4-8", "name": "Claude Opus 4.8", "context": 1048576, "description": "Top Opus-tier — complex reasoning & agentic coding", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools","vision","reasoning"]},
                    {"id": "claude-sonnet-4-6", "name": "Claude Sonnet 4.6", "context": 1048576, "description": "Best speed-intelligence balance for production", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools","vision"]},
                    {"id": "claude-haiku-4-5", "name": "Claude Haiku 4.5", "context": 204800, "description": "Fastest — high-volume, cost-sensitive tasks", "pricing": "token-package", "package_url": "/cloud/store?calc=1#calc-llm-grid", "capabilities": ["chat","tools"]}
                ]
            },
            {
                "id": "google", "name": "Google",
                "description": "Gemini 3.5 Flash and 3.1 Pro — agentic frontier. Token packages available.",
                "website": "https://ai.google.dev", "requires_byok": false, "icon": "google",
                "models": [
                    {"id": "gemini-3.5-flash", "name": "Gemini 3.5 Flash", "context": 1048576, "description": "GA — frontier agentic performance, 1M context", "pricing": "token-package", "capabilities": ["chat","vision","tools","reasoning","code-execution"]},
                    {"id": "gemini-3.1-pro", "name": "Gemini 3.1 Pro", "context": 1048576, "description": "Preview — advanced reasoning for complex tasks", "pricing": "token-package", "capabilities": ["chat","vision","tools","reasoning"]}
                ]
            },
            {
                "id": "groq", "name": "Groq",
                "description": "Ultra-fast inference on LPU. No Llama, no Mistral — only GPT-OSS and Qwen.",
                "website": "https://groq.com", "requires_byok": false, "icon": "groq",
                "models": [
                    {"id": "gpt-oss-120b", "name": "GPT-OSS 120B", "context": 131072, "description": "120B MoE — reasoning at 500 tok/s on LPU", "pricing": "token-package", "capabilities": ["chat","tools","reasoning"]},
                    {"id": "qwen-3.6-27b", "name": "Qwen 3.6-27B", "context": 131072, "description": "Best open 27B — 500 tok/s on Groq", "pricing": "token-package", "capabilities": ["chat","tools","reasoning"]},
                    {"id": "gpt-oss-20b", "name": "GPT-OSS 20B", "context": 131072, "description": "20B at 1000 tok/s — fastest option", "pricing": "token-package", "capabilities": ["chat","tools"]}
                ]
            },
            {
                "id": "generalbots", "name": "General Bots (Own GPU)",
                "description": "Open-weight models running on General Bots' own GPU infrastructure. Included in all plans.",
                "website": "https://generalbots.com.br", "requires_byok": false, "icon": "gb",
                "models": [
                    {"id": "qwen-3.6-27b", "name": "Qwen 3.6-27B", "context": 131072, "description": "27B parameters — best open model in its class", "pricing": "included", "capabilities": ["chat","tools","reasoning"]},
                    {"id": "deepseek-r1-distill-qwen", "name": "DeepSeek-R1-Distill-Qwen-1.5B", "context": 32768, "description": "Lightweight reasoning included in all plans", "pricing": "included", "capabilities": ["chat","reasoning"]},
                    {"id": "gpt-oss-20b", "name": "GPT-OSS 20B", "context": 32768, "description": "20B parameters on dedicated GPU", "pricing": "included", "capabilities": ["chat","tools"]}
                ]
            }
        ]
    })))
}

pub(crate) async fn handle_topup(
    State(service): State<Arc<SaasService>>,
    Json(body): Json<TopupBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let mut conn = service.pool().get()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("DB Connection: {e}")))?;

    // Retrieve default branch_id
    let branch_id = botbilling::get_bot_context(&service.billing_state.pool, &service.billing_state.get_default_bot);
    let effective_branch_id = if branch_id == Uuid::nil() {
        Uuid::nil()
    } else {
        branch_id
    };

    use std::str::FromStr;
    let decimal_amount = bigdecimal::BigDecimal::from_str(&format!("{:.2}", body.amount))
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid amount: {e}")))?;

    let zero = bigdecimal::BigDecimal::from(0);

    // Generate unique invoice number
    use rand::Rng;
    let mut rng = rand::rng();
    let num: u32 = rng.random_range(100_000..999_999);
    let invoice_num = format!("INV-TOPUP-{}", num);

    // Insert top-up invoice in database as paid
    let new_invoice_id = Uuid::new_v4();
    
    // Get contact name corresponding to the email, or use email as fallback
    use crate::schema_ext::crm_contacts::dsl::{crm_contacts, email, first_name, last_name};
    let contact_name = crm_contacts
        .filter(email.eq(&body.email))
        .select((first_name, last_name))
        .first::<(Option<String>, Option<String>)>(&mut conn)
        .map(|(fn_, ln_)| [fn_.unwrap_or_default(), ln_.unwrap_or_default()].join(" "))
        .map(|s| if s.trim().is_empty() { body.email.split('@').next().unwrap_or("Customer").to_string() } else { s })
        .unwrap_or_else(|_| body.email.split('@').next().unwrap_or("Customer").to_string());

    // Insert record in billing_invoices table
    diesel::insert_into(botbilling::schema::billing_invoices::table)
        .values((
            botbilling::schema::billing_invoices::id.eq(new_invoice_id),
            botbilling::schema::billing_invoices::branch_id.eq(effective_branch_id),
            botbilling::schema::billing_invoices::invoice_number.eq(&invoice_num),
            botbilling::schema::billing_invoices::customer_name.eq(&contact_name),
            botbilling::schema::billing_invoices::customer_email.eq(Some(body.email)),
            botbilling::schema::billing_invoices::status.eq("paid"),
            botbilling::schema::billing_invoices::issue_date.eq(chrono::Local::now().date_naive()),
            botbilling::schema::billing_invoices::due_date.eq(chrono::Local::now().date_naive()),
            botbilling::schema::billing_invoices::subtotal.eq(&decimal_amount),
            botbilling::schema::billing_invoices::tax_rate.eq(&zero),
            botbilling::schema::billing_invoices::tax_amount.eq(&zero),
            botbilling::schema::billing_invoices::discount_percent.eq(&zero),
            botbilling::schema::billing_invoices::discount_amount.eq(&zero),
            botbilling::schema::billing_invoices::total.eq(&decimal_amount),
            botbilling::schema::billing_invoices::amount_paid.eq(&decimal_amount),
            botbilling::schema::billing_invoices::amount_due.eq(&zero),
            botbilling::schema::billing_invoices::currency.eq("usd"),
            botbilling::schema::billing_invoices::notes.eq(Some("Account balance top-up via Special Offers".to_string())),
            botbilling::schema::billing_invoices::paid_at.eq(Some(chrono::Utc::now())),
            botbilling::schema::billing_invoices::created_at.eq(chrono::Utc::now()),
            botbilling::schema::billing_invoices::updated_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create topup invoice: {e}")))?;

    // Create payment in billing_payments to record the transaction
    let payment_id = Uuid::new_v4();
    let payment_num = format!("PAY-TOPUP-{}", num);
    diesel::insert_into(botbilling::schema::billing_payments::table)
        .values((
            botbilling::schema::billing_payments::id.eq(payment_id),
            botbilling::schema::billing_payments::branch_id.eq(effective_branch_id),
            botbilling::schema::billing_payments::invoice_id.eq(Some(new_invoice_id)),
            botbilling::schema::billing_payments::payment_number.eq(&payment_num),
            botbilling::schema::billing_payments::amount.eq(&decimal_amount),
            botbilling::schema::billing_payments::currency.eq("usd"),
            botbilling::schema::billing_payments::payment_method.eq("offline_topup"),
            botbilling::schema::billing_payments::status.eq("completed"),
            botbilling::schema::billing_payments::payer_name.eq(Some(&contact_name)),
            botbilling::schema::billing_payments::paid_at.eq(chrono::Utc::now()),
            botbilling::schema::billing_payments::created_at.eq(chrono::Utc::now()),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to register payment: {e}")))?;

    tracing::info!("Top-up success: Organization {} added credits of ${:.2}", body.org_id, body.amount);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "invoice_id": new_invoice_id,
        "invoice_number": invoice_num,
        "amount": decimal_amount.to_string(),
        "customer": contact_name,
    })))
}

// ─────────────────────────────────────────────────────────────────────────────
// BYOK: Bring Your Own Key — Encrypted Server-Side Storage
