use super::*;

/// The stable command list rendered into the prompt and used by discovery.
/// This now comes from the single source of truth in `crate::apps::commands`.
pub fn commands_list() -> Vec<&'static AppCommand> {
    crate::apps::commands::all_commands().iter().collect()
}
/// Full REST surface (method, path, one-line summary). Not injected — only
/// searched on demand through `api.find`, so the LLM knows the entire scale
/// without a giant system prompt. Keep entries grouped by domain.
pub fn all_endpoints() -> &'static [ApiEndpoint] {
    &[
        // Drive / files
        ApiEndpoint { method: "GET", path: "/api/files/list", summary: "List drive files for the bot" },
        ApiEndpoint { method: "POST", path: "/api/files/write", summary: "Upload a file (base64 content) into a folder" },
        ApiEndpoint { method: "POST", path: "/api/files/download", summary: "Download a file from a folder" },
        ApiEndpoint { method: "DELETE", path: "/api/files/delete", summary: "Delete a file" },
        ApiEndpoint { method: "POST", path: "/api/files/createFolder", summary: "Create a drive folder" },
        ApiEndpoint { method: "POST", path: "/api/files/copy", summary: "Copy a file" },
        ApiEndpoint { method: "POST", path: "/api/files/move", summary: "Move a file" },
        ApiEndpoint { method: "GET", path: "/api/files/search", summary: "Search drive files by name" },
        ApiEndpoint { method: "GET", path: "/api/files/recent", summary: "Most recent files" },
        ApiEndpoint { method: "GET", path: "/api/files/quota", summary: "Storage quota usage" },
        ApiEndpoint { method: "POST", path: "/api/files/upload-binary", summary: "Upload a raw binary file" },
        ApiEndpoint { method: "GET", path: "/api/files/buckets", summary: "List bot buckets" },
        ApiEndpoint { method: "GET", path: "/api/files/shared", summary: "List shared folders" },
        ApiEndpoint { method: "POST", path: "/api/files/share", summary: "Share a folder" },
        ApiEndpoint { method: "GET", path: "/api/files/trash", summary: "List trashed files" },
        ApiEndpoint { method: "POST", path: "/api/files/trash/restore", summary: "Restore a trashed file" },
        // Products / fiscal
        ApiEndpoint { method: "GET", path: "/api/products/items", summary: "List product catalog items" },
        ApiEndpoint { method: "POST", path: "/api/products/items", summary: "Create a product item" },
        ApiEndpoint { method: "GET", path: "/api/products/services", summary: "List registered services" },
        ApiEndpoint { method: "POST", path: "/api/products/services/:id/tax", summary: "Compute Brazilian taxes for a service" },
        ApiEndpoint { method: "GET", path: "/api/products/categories", summary: "List product categories" },
        ApiEndpoint { method: "GET", path: "/api/products/pricelists", summary: "List price lists" },
        ApiEndpoint { method: "GET", path: "/api/products/stats", summary: "Product statistics" },
        ApiEndpoint { method: "GET", path: "/api/products/low-stock", summary: "Low-stock items" },
        // Banking / cash flow
        ApiEndpoint { method: "GET", path: "/api/banking/transactions", summary: "List bank transactions" },
        ApiEndpoint { method: "POST", path: "/api/banking/transactions", summary: "Create a bank transaction" },
        ApiEndpoint { method: "GET", path: "/api/banking/platforms", summary: "List delivery platforms" },
        ApiEndpoint { method: "POST", path: "/api/banking/reconcile", summary: "Run reconciliation" },
        ApiEndpoint { method: "GET", path: "/api/banking/reports", summary: "Banking reports" },
        ApiEndpoint { method: "POST", path: "/api/banking/reconcile/match", summary: "Manual reconciliation match" },
        ApiEndpoint { method: "POST", path: "/api/banking/imports/cashflow", summary: "Import a monthly cash-flow CSV" },
        ApiEndpoint { method: "GET", path: "/api/banking/diagnosis", summary: "Cash-flow diagnosis of the branch" },
        // Billing / payroll
        ApiEndpoint { method: "GET", path: "/api/billing/payroll/months", summary: "Monthly invoice totals as a payroll basis" },
        // Settings: API keys & webhooks
        ApiEndpoint { method: "GET", path: "/api/user/api-keys", summary: "List API keys with scopes and last-used time" },
        ApiEndpoint { method: "POST", path: "/api/user/api-keys", summary: "Create an API key (name, scopes, expiry days)" },
        ApiEndpoint { method: "DELETE", path: "/api/user/api-keys/:key_id", summary: "Revoke an API key" },
        ApiEndpoint { method: "POST", path: "/api/user/api-keys/:key_id/rotate", summary: "Rotate an API key secret" },
        ApiEndpoint { method: "GET", path: "/api/user/webhooks", summary: "List webhook endpoints" },
        ApiEndpoint { method: "POST", path: "/api/user/webhooks", summary: "Register a webhook endpoint (URL, events)" },
        ApiEndpoint { method: "DELETE", path: "/api/user/webhooks/:webhook_id", summary: "Delete a webhook endpoint" },
        ApiEndpoint { method: "POST", path: "/api/user/webhooks/:webhook_id/test", summary: "Send a signed test event to a webhook" },
        ApiEndpoint { method: "GET", path: "/api/user/webhooks/:webhook_id/deliveries", summary: "List webhook delivery history" },
        // Sources / connectors
        ApiEndpoint { method: "GET", path: "/api/integrations/connectors", summary: "List data connectors with type, status and last sync" },
        ApiEndpoint { method: "POST", path: "/api/integrations/connectors", summary: "Create or update a data connector (database, API, SaaS)" },
        ApiEndpoint { method: "POST", path: "/api/integrations/connectors/:id/test", summary: "Test connectivity for a data connector" },
        ApiEndpoint { method: "POST", path: "/api/integrations/connectors/:id/sync", summary: "Trigger an immediate sync for a data connector" },
        ApiEndpoint { method: "DELETE", path: "/api/integrations/connectors/:id/disconnect", summary: "Remove a data connector and its vaulted credentials" },
        ApiEndpoint { method: "GET", path: "/api/integrations/connectors/templates", summary: "Connector type catalog for typed configuration forms" },
        // Integrations: tenant-scoped connection control plane (#939)
        ApiEndpoint { method: "GET", path: "/api/bots/:bot_id/integration-connections", summary: "List integration connections for a bot; credentials stay in Vault" },
        ApiEndpoint { method: "POST", path: "/api/bots/:bot_id/integration-connections", summary: "Create an integration connection (provider_slug, display_name, auth_kind); secret keys are stored in Vault only" },
        ApiEndpoint { method: "GET", path: "/api/bots/:bot_id/integration-connections/:connection_id", summary: "Get one integration connection; never returns vault path or credential material" },
        ApiEndpoint { method: "DELETE", path: "/api/bots/:bot_id/integration-connections/:connection_id", summary: "Delete an integration connection and purge its Vault credentials" },
        ApiEndpoint { method: "POST", path: "/api/bots/:bot_id/integration-connections/:connection_id/test", summary: "Validate stored credentials shape (no outbound call); outcome is recorded as unverified" },
        ApiEndpoint { method: "POST", path: "/api/bots/:bot_id/integration-connections/:connection_id/rotate", summary: "Rotate connection credentials in Vault and increment credential_version" },
        ApiEndpoint { method: "GET", path: "/api/bots/:bot_id/integration-connections/:connection_id/events", summary: "List the sanitized audit event trail of an integration connection" },
        ApiEndpoint { method: "POST", path: "/api/bots/:bot_id/integration-actions/invoke", summary: "Execute an implemented provider action (provider, action, params); credentials load from Vault and never return" },
        ApiEndpoint { method: "GET", path: "/api/apps/integrations/context", summary: "Resolve the default active bot id, name and integration-connections base URL for suite apps" },

        // Learning / LMS
        ApiEndpoint { method: "GET", path: "/api/learn/courses", summary: "List published courses with filters (category, difficulty, search)" },
        ApiEndpoint { method: "POST", path: "/api/learn/courses", summary: "Create a course draft" },
        ApiEndpoint { method: "POST", path: "/api/learn/courses/:id/enroll", summary: "Enroll the current user in a course" },
        ApiEndpoint { method: "GET", path: "/api/learn/progress", summary: "Current user learning progress and stats" },
        ApiEndpoint { method: "GET", path: "/api/learn/certifications", summary: "List certifications earned by the current user" },
        ApiEndpoint { method: "GET", path: "/api/learn/leaderboard", summary: "Leaderboard of learners by XP" },
        // Sales / CRM pipeline
        ApiEndpoint { method: "GET", path: "/api/sales/deals", summary: "List sales deals with stage, value, probability and owner" },
        ApiEndpoint { method: "POST", path: "/api/sales/deals", summary: "Create a sales deal (title, value, stage, expected close date)" },
        ApiEndpoint { method: "PATCH", path: "/api/sales/deals/:id", summary: "Update a deal (stage moves, value, probability, notes)" },
        ApiEndpoint { method: "DELETE", path: "/api/sales/deals/:id", summary: "Delete a deal" },
        ApiEndpoint { method: "GET", path: "/api/sales/contacts", summary: "List CRM contacts available for deals" },
        ApiEndpoint { method: "GET", path: "/api/sales/activities", summary: "List recent sales activities (calls, emails, meetings)" },
        ApiEndpoint { method: "GET", path: "/api/sales/funnel", summary: "Pipeline funnel summary grouped by stage with weighted value" },
        ApiEndpoint { method: "GET", path: "/api/sales/forecast", summary: "Weighted sales forecast from stage probabilities" },
        // Marketing campaigns, lists and templates
        ApiEndpoint { method: "GET", path: "/api/crm/campaigns", summary: "List marketing campaigns across channels" },
        ApiEndpoint { method: "POST", path: "/api/crm/campaigns", summary: "Create a marketing campaign (name, channel, content template)" },
        ApiEndpoint { method: "POST", path: "/api/crm/campaigns/:id/send", summary: "Send a campaign to its recipient list" },
        ApiEndpoint { method: "GET", path: "/api/crm/lists", summary: "List audience lists with member counts" },
        ApiEndpoint { method: "POST", path: "/api/crm/lists", summary: "Create an audience list (name, type, segment query)" },
        ApiEndpoint { method: "DELETE", path: "/api/crm/lists/:id", summary: "Delete an audience list" },
        ApiEndpoint { method: "GET", path: "/api/crm/templates", summary: "List channel templates (email, WhatsApp, social, SMS)" },
        ApiEndpoint { method: "POST", path: "/api/crm/templates", summary: "Create a channel template with variables" },
        ApiEndpoint { method: "DELETE", path: "/api/crm/templates/:id", summary: "Delete a channel template" },
        // VDI / remote desktop
        ApiEndpoint { method: "GET", path: "/api/desktop/connections", summary: "List active and saved remote desktop (VNC/RDP) connections for the user" },
        ApiEndpoint { method: "POST", path: "/api/desktop/connect", summary: "Register a remote desktop connection (VNC or RDP, credentials vaulted)" },
        ApiEndpoint { method: "POST", path: "/api/desktop/connections/kill", summary: "Terminate all active desktop sessions for the current user" },
        ApiEndpoint { method: "GET", path: "/api/desktop/connections/:id", summary: "Get a single desktop connection by id" },
        ApiEndpoint { method: "DELETE", path: "/api/desktop/connections/:id", summary: "Delete a desktop connection and terminate its session" },
        ApiEndpoint { method: "POST", path: "/api/desktop/health/rdp", summary: "Probe an RDP endpoint (host, port) and confirm an RDP server is listening" },
        ApiEndpoint { method: "POST", path: "/api/desktop/health/tcp", summary: "Probe TCP reachability of a host and port" },
        // Compliance
        ApiEndpoint { method: "GET", path: "/api/compliance/frameworks", summary: "List compliance frameworks with control counts" },
        ApiEndpoint { method: "POST", path: "/api/compliance/frameworks", summary: "Create a compliance framework (LGPD, GDPR, SOC 2, ISO 27001, PCI-DSS, custom)" },
        ApiEndpoint { method: "GET", path: "/api/compliance/frameworks/:framework_id", summary: "Framework detail with controls, evidence and coverage" },
        ApiEndpoint { method: "PUT", path: "/api/compliance/frameworks/:framework_id", summary: "Update a compliance framework" },
        ApiEndpoint { method: "POST", path: "/api/compliance/frameworks/:framework_id/archive", summary: "Archive a compliance framework" },
        ApiEndpoint { method: "GET", path: "/api/compliance/frameworks/:framework_id/export.csv", summary: "Audit-ready compliance scorecard CSV export" },
        ApiEndpoint { method: "POST", path: "/api/compliance/controls", summary: "Add a control to a compliance framework" },
        ApiEndpoint { method: "POST", path: "/api/compliance/evidence/attach", summary: "Attach drive evidence to a control" },
        ApiEndpoint { method: "POST", path: "/api/compliance/evidence/:evidence_id/approve", summary: "Approve attached evidence" },
        // Search / research
        ApiEndpoint { method: "GET", path: "/api/ui/search", summary: "Unified search across business entities" },
        ApiEndpoint { method: "POST", path: "/api/ui/research/web/search", summary: "Web search (DuckDuckGo)" },
        ApiEndpoint { method: "POST", path: "/api/ui/research/web/summarize", summary: "Summarize web search results" },
        ApiEndpoint { method: "POST", path: "/api/ui/research/web/deep", summary: "Deep research on a topic" },
        ApiEndpoint { method: "GET", path: "/api/ui/research/web/instant", summary: "Instant answer from the web" },
        // Vibe / project workbench
        ApiEndpoint { method: "POST", path: "/api/vibe/run", summary: "Run an authenticated Vibe agent request against a selected project" },
        ApiEndpoint { method: "GET", path: "/api/vibe/runs", summary: "List Vibe agent runs" },
        ApiEndpoint { method: "GET", path: "/api/vibe/projects", summary: "List the user's Vibe projects" },
        ApiEndpoint { method: "GET", path: "/api/vibe/projects/:project_id", summary: "Get a Vibe project" },
        ApiEndpoint { method: "GET", path: "/api/vibe/projects/:project_id/preview", summary: "Resolve a selected project's live preview" },
        // Catalog / apps
        ApiEndpoint { method: "GET", path: "/api/apps/catalog", summary: "Suite application catalog" },
        ApiEndpoint { method: "GET", path: "/api/apps/integrations/catalog", summary: "Search the integration provider and action catalog" },
        ApiEndpoint { method: "GET", path: "/api/apps/integrations/catalog/:provider", summary: "Get one integration provider and its action catalog" },
        ApiEndpoint { method: "GET", path: "/api/catalog/products", summary: "Public product catalog" },
        ApiEndpoint { method: "GET", path: "/api/catalog/plans", summary: "Public plans catalog" },
        ApiEndpoint { method: "GET", path: "/api/catalog/prices.json", summary: "Price list (JSON-LD)" },
        // Organization / cloud
        ApiEndpoint { method: "GET", path: "/api/organizations/current", summary: "Current organization profile" },
        ApiEndpoint { method: "PUT", path: "/api/organizations/current", summary: "Update organization profile" },
        ApiEndpoint { method: "GET", path: "/api/organizations/current/stats", summary: "Organization statistics" },
        ApiEndpoint { method: "POST", path: "/api/cloud/auth/signup", summary: "Create a cloud workspace account" },
        ApiEndpoint { method: "POST", path: "/api/cloud/auth/login", summary: "Log into the cloud" },
        ApiEndpoint { method: "GET", path: "/api/cloud/domains", summary: "List bot domain mappings" },
        ApiEndpoint { method: "POST", path: "/api/cloud/domains", summary: "Map a domain to a bot" },
        ApiEndpoint { method: "GET", path: "/api/domains/resolve", summary: "Resolve a hostname to a bot" },
        ApiEndpoint { method: "POST", path: "/api/autotask/classify", summary: "Classify a plain-language intent into an automation plan" },
        ApiEndpoint { method: "POST", path: "/api/autotask/compile", summary: "Compile an automation plan into a BASIC script" },
        ApiEndpoint { method: "POST", path: "/api/autotask/decide", summary: "Decide immediate vs scheduled execution mode" },
        ApiEndpoint { method: "POST", path: "/api/autotask/execute", summary: "Execute an automation plan (compiled BASIC script)" },
        ApiEndpoint { method: "POST", path: "/api/autotask/create-and-execute", summary: "Turn a plain-language request into an automated task and run it" },
        ApiEndpoint { method: "GET", path: "/api/autotask/tasks", summary: "List AutoTask runs and their status" },
        ApiEndpoint { method: "GET", path: "/api/autotask/stats", summary: "AutoTask statistics: runs, success rate, pending approvals" },
        ApiEndpoint { method: "POST", path: "/api/autotask/tasks/:task_id/approve", summary: "Approve a pending automated task" },
        ApiEndpoint { method: "POST", path: "/api/autotask/tasks/:task_id/cancel", summary: "Cancel a queued or running automated task" },
        // Attendant / human handoff
        ApiEndpoint { method: "POST", path: "/api/attendant/attachments", summary: "Upload an attachment to an attendant conversation" },
        ApiEndpoint { method: "GET", path: "/api/attendant/attachments/:id", summary: "Download an attendant conversation attachment" },
        // Meet / recording
        ApiEndpoint { method: "POST", path: "/api/meet/rooms/:id/recording/start", summary: "Start recording a meeting (host/co-host only)" },
        ApiEndpoint { method: "POST", path: "/api/meet/rooms/:id/recording/stop", summary: "Stop the active meeting recording" },
        ApiEndpoint { method: "GET", path: "/api/meet/rooms/:id/recordings", summary: "List recordings for a meeting room" },
        ApiEndpoint { method: "GET", path: "/api/meet/recordings/:recording_id/file", summary: "Stream or download a meeting recording file" },
        // Player / playlists
        ApiEndpoint { method: "GET", path: "/api/player/playlists", summary: "List media playlists for the current user" },
        ApiEndpoint { method: "POST", path: "/api/player/playlists", summary: "Create a media playlist" },
        ApiEndpoint { method: "GET", path: "/api/player/playlists/:id", summary: "Get a playlist with its items" },
        ApiEndpoint { method: "PUT", path: "/api/player/playlists/:id", summary: "Rename a media playlist" },
        ApiEndpoint { method: "DELETE", path: "/api/player/playlists/:id", summary: "Delete a media playlist" },
        ApiEndpoint { method: "POST", path: "/api/player/playlists/:id/items", summary: "Add a media item to a playlist" },
        ApiEndpoint { method: "DELETE", path: "/api/player/playlists/:id/items/:item_id", summary: "Remove a media item from a playlist" },
        ApiEndpoint { method: "PUT", path: "/api/player/playlists/:id/reorder", summary: "Reorder playlist items" },
        ApiEndpoint { method: "GET", path: "/api/player/playlists/:id/analytics", summary: "Playlist analytics: counts, total items, recent activity" },
        ApiEndpoint { method: "POST", path: "/api/player/playbacks", summary: "Record a media playback event (play, pause, complete)" },
        // Dashboards / analytics
        ApiEndpoint { method: "GET", path: "/api/dashboards", summary: "List dashboards with their widgets" },
        ApiEndpoint { method: "POST", path: "/api/dashboards", summary: "Create a dashboard" },
        ApiEndpoint { method: "GET", path: "/api/dashboards/:id", summary: "Get a dashboard with its widgets and filters" },
        ApiEndpoint { method: "PUT", path: "/api/dashboards/:id", summary: "Rename or reconfigure a dashboard" },
        ApiEndpoint { method: "DELETE", path: "/api/dashboards/:id", summary: "Delete a dashboard" },
        ApiEndpoint { method: "POST", path: "/api/dashboards/:id/widgets", summary: "Add a widget to a dashboard" },
        ApiEndpoint { method: "PUT", path: "/api/dashboards/:id/widgets/:widget_id", summary: "Update a widget title, size or chart configuration" },
        ApiEndpoint { method: "DELETE", path: "/api/dashboards/:id/widgets/:widget_id", summary: "Remove a widget from a dashboard" },
        ApiEndpoint { method: "GET", path: "/api/dashboards/sources", summary: "List dashboard data sources" },
    ]
}
/// Compact system-prompt fragment teaching the `__api_call__` contract and
/// the routing rule, shortened to the current user's role so admin-only
/// commands are never proposed to regular users.
/// Compact system-prompt fragment teaching the `__api_call__` contract and the
/// discovery round-trip. The injected surface is intentionally tiny — instead
/// of dumping every command (there are now 1000+ harvested endpoints), the LLM
/// is taught to DISCOVER the exact command/endpoint with `api.find`, then EXECUTE
/// it with `api.exec` (or a named curated command). This keeps the prompt small
/// while unlocking the full action surface on every channel.
pub fn api_command_instructions(role: &str) -> String {
    let mut lines = Vec::with_capacity(24);
    lines.push("---".to_string());
    lines.push("## API Commands".to_string());
    lines.push(
        "You can call backend commands to read or change business data (fiscal, banking, drive files) \
         or search the web. Answer general-knowledge questions (trivia, definitions, opinions) DIRECTLY \
         with no command."
            .to_string(),
    );
    lines.push(format!("Your role: {role}. Only issue commands you are allowed to use."));
    lines.push("To call one, make the FIRST line of your reply exactly a JSON object:".to_string());
    lines.push("{\"__api_call__\": {\"name\": \"<command>\", \"params\": {<named params>}, \"compose\": true}}".to_string());
    lines.push("then add a short user-facing note after it.".to_string());

    // Discovery contract — never list the full surface in the prompt.
    lines.push("Discovery (2-step, prefer this over guessing a command):".to_string());
    lines.push("  1. `api.find` with params {query}: returns the matching curated command OR a harvested endpoint (kind, name/method/path, summary).".to_string());
    lines.push("  2. Run it: `api.exec` with params {method, path, params} executes any registered endpoint; or use the curated command name when returned.".to_string());

    // Only the most common commands are named explicitly to avoid prompt bloat.
    lines.push("Common commands you may call directly:".to_string());
    for name in [
        "apps.find", "api.find", "api.exec", "tasks.autotask.create", "tasks.autotask.list",
        "vibe.project.change", "integrations.actions.list", "service.tax", "banking.diagnosis", "drive.list",
    ] {
        if let Some(cmd) = command_by_name(name) {
            let params = cmd
                .params
                .iter()
                .map(|(n, d)| format!("{n}: {d}"))
                .collect::<Vec<_>>()
                .join("; ");
            lines.push(format!("- {name}: {summary} (params: {params})", summary = cmd.summary));
        }
    }
    lines.push(
        "Rules: use compose=true when the answer needs prose from the data. Never mention JSON or \
         commands to the user. Discovery: api.find matches command NAMES and SUMMARIES by keyword, not \
         free-form intent, so search with domain words such as 'task', 'automation', 'drive', 'invoice' \
         rather than the user's own phrasing. When the command is listed above, call it directly by name \
         instead of searching. To build a new automation from a plain-language request, call \
         tasks.autotask.create with the request as the intent parameter."
            .to_string(),
    );
    lines.join("\n")
}
/// Shared keyword scorer: how many query tokens appear in the haystack.
pub(super) fn score_tokens(hay: &str, tokens: &[String]) -> i32 {
    tokens.iter().filter(|t| hay.contains(t.as_str())).count() as i32
}
pub(super) fn tokens_of(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split_whitespace()
        .map(|t| t.to_string())
        .collect()
}
/// App registry search used by the `apps.find` command (covers all apps).
pub(super) fn find_apps(query: &str) -> Vec<Value> {
    let q = tokens_of(query);
    if q.is_empty() {
        return Vec::new();
    }
    let apps = registry::all_apps();
    let mut scored: Vec<(i32, &registry::AppDefinition)> = apps
        .iter()
        .filter_map(|app| {
            let hay = format!(
                "{} {} {} {}",
                app.title.to_lowercase(),
                app.id.to_lowercase(),
                app.description.to_lowercase(),
                app.keywords.to_lowercase()
            );
            let score = score_tokens(&hay, &q);
            if score > 0 {
                Some((score, app))
            } else {
                None
            }
        })
        .collect();
    scored.sort_by_key(|a| std::cmp::Reverse(a.0));
    scored
        .into_iter()
        .take(3)
        .map(|(_, app)| {
            json!({
                "id": app.id,
                "title": app.title,
                "description": app.description,
                "url": app.url,
            })
        })
        .collect()
}
