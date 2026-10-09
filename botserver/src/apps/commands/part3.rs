use super::*;

/// Deep-link params for an app id (empty when none are declared).
pub fn deep_link_params_for_app(app_id: &str) -> &'static [DeepLinkParam] {
    APP_DEEP_LINKS
        .iter()
        .find(|(id, _)| *id == app_id)
        .map(|(_, params)| *params)
        .unwrap_or(&[])
}
/// Every command across all applications.
pub static ALL_COMMANDS: &[AppCommand] = &[
    // ——— AI ———
    cmd("chat", "chat.resume", "Resume conversation", "Resume or start a chat with the assistant.", &[], None, false),
    cmd("vibe", "vibe.runs.list", "List assistant runs", "List generative assistant runs and workflows.", &[("limit", "optional max results")], Some("app://vibe?run_id={run_id}"), false),
    cmd("vibe", "vibe.project.change", "Change a Vibe project", "Run an authenticated Vibe agent request against the referenced project workspace.", &[("project_id", "selected Vibe project UUID"), ("project_name", "selected project name"), ("intent", "requested code or project change")], None, false),
    cmd("research", "research.web.search", "Search the web", "Search the web (DuckDuckGo) for current facts, news or prices.", &[("query", "the search terms"), ("max_results", "optional 1-25")], Some("app://research?q={query}"), false),
    cmd("research", "research.discover", "Deep research", "Performs deep research on a topic across web and knowledge bases.", &[("topic", "the topic")], None, false),
    cmd("video", "video.projects.list", "List video projects", "List AI video editing/generation projects.", &[], Some("app://video?project_id={project_id}"), false),
    cmd("jukebox", "jukebox.generate", "Create a song", "Create a complete song or instrumental from a description, style and optional lyrics.", &[("description", "the song brief"), ("lyrics", "optional structured lyrics"), ("duration", "10-600 seconds")], Some("app://jukebox?job_id={job_id}"), false),
    cmd("vision", "vision.analyze", "Analyze an image", "Run computer vision analysis on an image (labels, QR codes, description).", &[("image", "image url or key")], None, false),
    cmd("learn", "learn.courses.list", "List courses", "List available learning courses and assessments.", &[], Some("app://learn?course_id={course_id}"), false),
    // ——— Office ———
    cmd("mail", "mail.list", "List emails", "List emails from the unified inbox.", &[("folder", "optional folder"), ("limit", "optional")], Some("app://mail?message_id={message_id}"), false),
    cmd("mail", "mail.send", "Send an email", "Send an email message.", &[("to", "recipient"), ("subject", "subject"), ("body", "body")], None, true),
    cmd("calendar", "calendar.events.list", "List events", "List calendar events.", &[("start", "start date"), ("end", "end date")], Some("app://calendar?event_id={event_id}"), false),
    cmd("meet", "meet.recordings.list", "List recordings", "List meeting recordings and transcriptions.", &[], Some("app://meet?meeting_id={meeting_id}"), false),
    cmd("meet", "meet.recordings.start", "Start recording", "Start recording a meeting (host only).", &[("room_id", "meeting room id")], None, false),
    cmd("meet", "meet.recordings.stop", "Stop recording", "Stop the active recording of a meeting (host only).", &[("room_id", "meeting room id")], None, false),
    cmd("docs", "docs.list", "List documents", "List documents in the workspace.", &[], Some("app://docs?file={file}"), false),
    cmd("sheet", "sheet.open", "Open spreadsheet", "Open a spreadsheet and read cells.", &[("file_id", "spreadsheet id"), ("cell", "optional A1 cell")], Some("app://sheet?file_id={file_id}&cell={cell}"), false),
    cmd("tasks", "tasks.list", "List tasks", "List tasks and their status.", &[("filter", "all/active/completed")], Some("app://tasks?task_id={task_id}"), false),
    cmd("tasks", "tasks.autotask.create", "Create automated task", "Turn a plain-language request into an automated task and run it (classify, plan, generate BASIC script, execute).", &[("intent", "what should be automated")], Some("app://tasks"), true),
    cmd("tasks", "tasks.autotask.list", "List automated tasks", "List AutoTask runs, their status and pending approvals.", &[("filter", "all/active/completed")], Some("app://tasks?task_id={task_id}"), false),
    cmd("tasks", "tasks.autotask.approve", "Approve automated task", "Approve a pending automated task for execution.", &[("task_id", "task id")], Some("app://tasks?task_id={task_id}"), true),
    cmd("tasks", "tasks.autotask.cancel", "Cancel automated task", "Cancel a queued or running automated task.", &[("task_id", "task id")], Some("app://tasks?task_id={task_id}"), true),
    cmd("tasks", "tasks.autotask.stats", "AutoTask statistics", "Automation statistics: runs, success rate, pending approvals.", &[], Some("app://tasks"), false),
    cmd("project", "project.tasks.list", "List project tasks", "List project phases, tasks and deliverables.", &[("project_id", "optional project")], Some("app://project?project_id={project_id}"), false),
    cmd("plan", "plan.okrs.list", "List OKRs", "List strategic objectives and key results.", &[], None, false),
    cmd("goals", "goals.okrs.list", "List goals", "List objectives and key results with progress.", &[], Some("app://goals?okr_id={okr_id}"), false),
    cmd("minutes", "minutes.list", "List minutes", "List meeting minutes and action items.", &[], Some("app://minutes?minutes_id={minutes_id}"), false),
    cmd("timeclock", "timeclock.entries", "List time entries", "List time tracking and attendance entries.", &[], Some("app://timeclock?entry_id={entry_id}"), false),
    cmd("templates", "templates.list", "List templates", "List reusable document and app templates.", &[], Some("app://templates?template_id={template_id}"), false),
    cmd("canvas", "canvas.open", "Open canvas", "Open a creative canvas or whiteboard.", &[("id", "canvas id")], Some("app://canvas?canvas_id={id}"), false),
    cmd("paper", "paper.notes.list", "List notes", "List rich notes and documents.", &[], Some("app://paper?note_id={note_id}"), false),
    cmd("slides", "slides.list", "List presentations", "List presentation decks.", &[], Some("app://slides?deck_id={deck_id}"), false),
    // ——— Business ———
    cmd("crm", "crm.people.list", "List people", "List CRM people/contacts and their pipeline stage.", &[], Some("app://crm?person_id={person_id}"), false),
    cmd("crm", "crm.people.search", "Search people", "Search people by name and return a deep link to the matched record.", &[("query", "name to search")], Some("app://crm?person_id={person_id}"), false),
    cmd("crm", "crm.pipeline.forecast", "Sales forecast", "Report the weighted pipeline forecast and funnel conversion rates for the coming months.", &[("periods", "optional number of months ahead (default 3)")], Some("app://crm"), false),
    cmd("crm", "crm.leads.create", "Create lead", "Capture a new lead with contact data and value (creates/links contact and account).", &[("title", "short lead title"), ("first_name", "prospect first name"), ("last_name", "prospect last name"), ("email", "prospect email"), ("company", "prospect company"), ("value", "estimated value"), ("currency", "currency code, default USD")], None, false),
    cmd("crm", "crm.leads.report", "Pipeline report", "Summarize leads by stage with counts and total value.", &[], Some("app://crm"), false),
    cmd("crm", "crm.deal.create", "Create deal", "Create a deal in the pipeline, creating/linking the contact (by email) and account (by company) when given.", &[("title", "deal title"), ("value", "deal value"), ("currency", "currency code, default USD"), ("stage", "initial stage, default new"), ("email", "contact e-mail, links or creates the contact"), ("company", "company name, links or creates the account")], Some("app://crm"), false),
    cmd("crm", "crm.deal.move-stage", "Move deal stage", "Move a deal to another pipeline stage (same rule as the kanban drag-and-drop).", &[("deal_id", "deal uuid"), ("stage", "target stage name")], Some("app://crm"), false),
    cmd("crm", "crm.contact.create", "Create contact", "Capture a contact, deduplicated by e-mail inside the workspace.", &[("email", "contact e-mail"), ("first_name", "first name"), ("last_name", "last name"), ("phone", "phone"), ("company", "company name")], Some("app://crm?person_id={contact_id}"), false),
    cmd("people", "people.list", "List people", "List contacts and leads.", &[], Some("app://people?person_id={person_id}"), false),
    cmd("people", "people.search", "Search people", "Search contacts/leads by name, with deep link to the record.", &[("query", "name or email")], Some("app://people?person_id={person_id}"), false),
    cmd("billing", "billing.invoice.list", "List invoices", "List invoices, quotes and payment status.", &[], Some("app://billing?invoice_id={invoice_id}"), false),
    cmd("billing", "billing.subscription.status", "Subscription status", "Report the current plan and subscription status.", &[], None, false),
    cmd("products", "products.items.list", "List products", "List product catalog items and services.", &[("category", "optional category")], Some("app://products?product_id={product_id}"), false),
    cmd("tickets", "tickets.list", "List tickets", "List support tickets and their status.", &[], Some("app://tickets?ticket_id={ticket_id}"), false),
    cmd("tickets", "tickets.create", "Create ticket", "Create a support ticket.", &[("subject", "subject"), ("priority", "priority")], None, true),
    cmd("banking", "banking.transactions.list", "List transactions", "List bank transactions for a period.", &[("period", "optional YYYY-MM")], Some("app://banking?transaction_id={transaction_id}"), false),
    cmd("banking", "banking.reconcile", "Reconcile account", "Run bank reconciliation.", &[], None, true),
    cmd("pos", "pos.sales.list", "List sales", "List point-of-sale orders and sales.", &[], Some("app://pos?order_id={order_id}"), false),
    cmd("retail", "retail.stock.list", "List stock", "List retail inventory and stock levels.", &[], Some("app://retail?product_id={product_id}"), false),
    cmd("hr", "hr.employees.list", "List employees", "List employees, onboarding and requests.", &[], Some("app://hr?employee_id={employee_id}"), false),
    cmd("tax", "tax.calculate", "Calculate tax", "Calculate Brazilian service taxes for a service value or registered service.", &[("service", "optional service name or id"), ("value", "the service amount")], None, false),
    cmd("tax", "tax.documents.list", "List tax documents", "List Brazilian tax documents (NF-e, NFS-e, CT-e).", &[], Some("app://tax?doc_id={doc_id}"), false),
    cmd("compliance", "compliance.audit.list", "List audit log", "List compliance scans, audit log and evidence.", &[], Some("app://compliance?audit_id={audit_id}"), false),
    cmd("compliance", "compliance.framework.list", "List frameworks", "List compliance frameworks with their controls and coverage.", &[], Some("app://compliance?framework_id={framework_id}"), false),
    cmd("compliance", "compliance.framework.create", "Create framework", "Create a compliance framework (LGPD, GDPR, SOC 2, ISO 27001, PCI-DSS or custom) with name, version and description.", &[("name", "framework name"), ("version", "framework version (optional)")], Some("app://compliance?framework_id={framework_id}"), false),
    cmd("compliance", "compliance.control.add", "Add control", "Add a control (id, title, category, mandatory) to a compliance framework.", &[("framework_id", "framework uuid"), ("control_id", "control identifier"), ("title", "control title")], None, false),
    cmd("compliance", "compliance.evidence.attach", "Attach evidence", "Attach a drive artifact as evidence to a compliance control.", &[("control_id", "control uuid"), ("file_path", "drive path")], None, false),
    cmd("compliance", "compliance.report.export", "Export scorecard", "Export an audit-ready compliance scorecard (CSV) for a framework.", &[("framework_id", "framework uuid")], Some("app://compliance?framework_id={framework_id}"), false),
    cmd("kyc", "kyc.verifications.list", "List verifications", "List KYC identity verifications.", &[], Some("app://kyc?verification_id={verification_id}"), false),
    cmd("fraud", "fraud.cases.list", "List cases", "List anti-fraud cases and assessments.", &[], Some("app://fraud?case_id={case_id}"), false),
    cmd("handoff", "handoff.queue.list", "List queue", "List human handoff and escalation queue.", &[], Some("app://handoff?handoff_id={handoff_id}"), false),
    cmd("campaigns", "campaigns.list", "List campaigns", "List marketing campaigns across channels.", &[], Some("app://campaigns?campaign_id={campaign_id}"), false),
    cmd("lists", "lists.list", "List lists", "List structured data lists.", &[], Some("app://lists?list_id={list_id}"), false),
    cmd("social", "social.feed.list", "List feed", "List social media feed.", &[], Some("app://social?post_id={post_id}"), false),
    cmd("attendant", "attendant.queue.list", "List queue", "List the human attendant queue.", &[], Some("app://attendant?queue_id={queue_id}"), false),
    cmd("analytics", "analytics.dashboard", "Get dashboard", "Get business analytics dashboard metrics.", &[], Some("app://analytics?report_id={report_id}"), false),
    cmd("analytics", "dashboards.list", "List dashboards", "List dashboards with their widgets.", &[], Some("app://analytics"), false),
    cmd("analytics", "dashboards.get", "Get dashboard", "Get a dashboard with its widgets and layout.", &[("dashboard_id", "dashboard uuid")], Some("app://analytics"), false),
    cmd("analytics", "dashboards.widgets.update", "Edit widget", "Update a widget's title, size or chart configuration.", &[("dashboard_id", "dashboard uuid"), ("widget_id", "widget uuid")], Some("app://analytics"), false),
    cmd("monitoring", "monitoring.health", "Check health", "System health: services, resources and alerts.", &[], None, false),
    // ——— Dev ———
    cmd("editor", "editor.file.open", "Open file", "Open a file in the code editor.", &[("file", "file path")], Some("app://editor?file_id={file}"), false),
    cmd("bas-editor", "bas.script.open", "Open script", "Open a BASIC script for editing.", &[("script", "script name")], Some("app://bas-editor?script_id={script}"), false),
    cmd("database", "database.tables.list", "List tables", "List database tables.", &[], Some("app://database?table={table}"), false),
    cmd("database", "database.query", "Run read-only query", "Run a read-only SQL query against a table.", &[("table", "table name"), ("limit", "optional")], None, true),
    cmd("database", "database.connections.create", "Create database connection", "Create or update an external database connection (conn-*) used by the TABLE keyword; the password is a sensitive key and is vaulted, never written to config.csv.", &[("name", "connection name"), ("server", "database host"), ("database", "database name"), ("username", "optional user"), ("password", "optional password (vaulted)"), ("port", "optional port"), ("driver", "optional postgres/mysql/mariadb/mssql")], None, false),
    cmd("browser", "browser.session.open", "Open browser", "Open an embedded browser session to a URL.", &[("url", "the url")], Some("app://browser?url={url}"), false),
    cmd("integrations", "integrations.connectors.list", "List connectors", "List external system connectors and webhooks.", &[], Some("app://integrations?connector_id={connector_id}"), false),
    cmd("integrations", "integrations.catalog.search", "Search integration catalog", "Search integration providers by text, category, or implementation status.", &[("q", "optional provider or capability query"), ("category", "optional category"), ("status", "optional built/partial/planned/unsupported status")], None, false),
    cmd("integrations", "integrations.actions.list", "List provider actions", "List safe action metadata for an integration provider without exposing authentication metadata.", &[("provider", "provider id, e.g. aws or zendesk")], None, false),
    cmd("integrations", "integrations.invoke", "Invoke integration action", "Execute one implemented provider action (e.g. aws.s3.objects.list) through the tenant's active connection; credentials never appear in the output.", &[("provider", "provider catalog id, e.g. aws"), ("action", "catalog action key, e.g. s3.objects.list"), ("params", "JSON object with the action parameters")], None, false),
    cmd("integrations", "integrations.sources.list", "List integration sources", "List connected data sources, models and MCP servers.", &[], Some("app://integrations"), false),
    cmd("integrations", "integrations.connectors.test", "Test connector", "Run a live connectivity check on a configured connector.", &[("connector_id", "connector uuid")], None, false),
    cmd("integrations", "integrations.connectors.sync", "Sync connector", "Trigger an immediate sync for a configured connector.", &[("connector_id", "connector uuid")], None, false),
    cmd("integrations", "integrations.connectors.create", "Create connector", "Create a provider connection after its secure adapter is available.", &[("provider", "provider catalog id"), ("name", "connection name")], None, false),
    cmd("integrations", "integrations.api-keys.list", "List API keys", "List API keys with scopes, expiry and last-used time.", &[], None, false),
    cmd("integrations", "integrations.api-keys.create", "Create API key", "Create an API key with name, optional comma-separated scopes and optional expiry in days.", &[("name", "key name"), ("scopes", "optional comma-separated scopes"), ("expires_in_days", "optional expiry in days")], None, false),
    cmd("integrations", "integrations.api-keys.revoke", "Revoke API key", "Revoke an API key by id.", &[("key_id", "api key uuid")], None, true),
    cmd("integrations", "integrations.webhooks.list", "List webhooks", "List webhook endpoints with their events and status.", &[], None, false),
    cmd("integrations", "integrations.webhooks.create", "Add webhook", "Register a webhook endpoint with URL and optional comma-separated events.", &[("url", "https endpoint"), ("events", "optional comma-separated events")], None, false),
    cmd("integrations", "integrations.webhooks.test", "Test webhook", "Send a signed test event to a webhook endpoint.", &[("webhook_id", "webhook uuid")], None, false),
    cmd("integrations", "integrations.webhooks.deliveries", "Webhook deliveries", "List the delivery history of a webhook endpoint.", &[("webhook_id", "webhook uuid")], None, false),
    cmd("vdi", "vdi.list", "List remote desktops", "List saved remote desktop (VNC/RDP) connections for the user.", &[], None, false),
    cmd("vdi", "vdi.connect", "Connect to remote desktop", "Register and open a VNC or RDP remote desktop session to a host.", &[("host", "target host"), ("port", "target port"), ("protocol", "vnc or rdp")], Some("app://desktop/vdi.html"), false),
    cmd("vdi", "vdi.health.rdp", "Probe RDP server", "Check whether an RDP server is listening on host:port.", &[("host", "target host"), ("port", "target port")], None, false),
    cmd("workspace", "workspace.pages.list", "List pages", "List workspace pages.", &[], Some("app://workspace?page_id={page_id}"), false),
    cmd("admin", "admin.users.list", "List users", "List organization users, roles and groups.", &[], None, true),
    cmd("settings", "settings.read", "Read settings", "Read user and workspace settings.", &[], None, false),
    // ——— System ———
    cmd("drive", "drive.list", "List files", "List drive files for a folder.", &[("path", "optional folder")], Some("app://drive?path={path}"), false),
    cmd("drive", "drive.search", "Search files", "Search drive files by name.", &[("query", "file name")], Some("app://drive?path={path}"), false),
    cmd("vdi", "vdi.connect", "Connect VDI", "Connect a virtual desktop session.", &[], None, false),
    cmd("biometry", "biometry.status", "Biometry status", "Report biometric verification status.", &[], None, false),
    cmd("player", "player.streams.list", "List streams", "List media streams.", &[], Some("app://player?media_id={media_id}"), false),
    cmd("player", "player.playlists.list", "List playlists", "List the user's media playlists.", &[], Some("app://player"), false),
    cmd("player", "player.playlists.create", "Create playlist", "Create a media playlist.", &[("name", "playlist name")], Some("app://player"), false),
    cmd("player", "player.playlists.add", "Add to playlist", "Add a media stream to a playlist.", &[("playlist_id", "playlist uuid"), ("media_id", "stream id")], Some("app://player"), false),
    cmd("player", "player.playlists.play", "Play playlist", "Start playback of a playlist.", &[("playlist_id", "playlist uuid")], Some("app://player"), false),
    // ——— Legacy core commands (kept for backward compatibility: the chat
    // prompt and executors reference these exact names) ———
    cmd("tax", "service.tax", "Calculate service taxes", "Compute Brazilian service taxes for a service value or registered service (IRPJ, CSLL, PIS/COFINS, ISS).", &[("service", "optional service name or id"), ("value", "the service amount; required when no service is given")], None, false),
    cmd("banking", "banking.diagnosis", "Financial diagnosis", "Cash-flow health of the account: revenue, expenses, net, pending reconciliation and tax rates.", &[("period", "optional YYYY-MM month filter")], None, false),
    cmd("banking", "banking.import", "Import cash-flow", "Import a month's cash-flow sheet (CSV stored in the bot drive) into the financial model.", &[("file_key", "drive path of the CSV"), ("period", "optional YYYY-MM")], None, false),
    cmd("drive", "drive.write", "Store a file", "Store a file (e.g. an invoice) in the bot drive under a folder path.", &[("path", "folder/file name"), ("content_base64", "the file bytes in base64")], None, false),
    cmd("drive", "drive.file", "Organize a file", "Organize a stored drive file (e.g. an attached invoice) into its folder.", &[("from", "current drive path"), ("to", "destination folder path")], None, false),
    cmd("drive", "drive.archive", "Archive invoices", "Move invoice-like files from the drive inbox into an archive folder to keep it clean.", &[("source", "optional source folder (empty = whole drive)"), ("destination", "archive folder path")], None, false),
    cmd("billing", "payroll.diagnosis", "Payroll diagnosis", "Aggregate a branch's monthly invoice totals as a payroll financial basis.", &[("period", "optional YYYY-MM month filter")], None, false),
    cmd("research", "web.search", "Search the web", "Search the web (DuckDuckGo) for current facts, news or prices.", &[("query", "the search terms"), ("max_results", "optional 1-25")], None, false),
    cmd("", "apps.find", "Find an app", "Find a suite application by a description of what the user wants to do.", &[("query", "what the user wants to accomplish")], None, false),
    cmd("", "api.find", "Find an API command", "Discover which backend command or endpoint matches a described need.", &[("query", "the described need")], None, false),
    cmd("", "api.exec", "Run any API endpoint", "Execute any registered backend endpoint on demand (create, update, search or delete data in any app).", &[("method", "HTTP method: GET, POST, PUT, PATCH or DELETE"), ("path", "registered endpoint path, e.g. /api/crm/contacts"), ("params", "JSON object of path params and body/query fields")], None, false),
];
/// Commands declared for an app id (ordered by declaration).
pub fn commands_for_app(app_id: &str) -> Vec<&'static AppCommand> {
    ALL_COMMANDS.iter().filter(|c| c.app == app_id).collect()
}
/// ALL commands (the catalogued LLM surface).
pub fn all_commands() -> &'static [AppCommand] {
    ALL_COMMANDS
}
/// Look up a command by its fully-qualified name.
pub fn command_by_name(name: &str) -> Option<&'static AppCommand> {
    ALL_COMMANDS.iter().find(|c| c.name == name)
}
/// Per-app UI automation sequence hints, injected into the web `__ui_plan__`
/// prompt so the LLM plans realistic navigation for each application.
pub static UI_SEQUENCE_HINTS: &[(&str, &str)] = &[
    ("crm", "open → click the relevant tab (Contacts/Deals/Pipeline) → use search or select a row → click Edit/New"),
    ("people", "open → contacts list auto-loads → select a person → panel shows the record"),
    ("billing", "open → invoices list auto-loads → use the filter/search → click an invoice to view details"),
    ("products", "open → items grid auto-loads → filter by category/status → click a row to edit"),
    ("tickets", "open → ticket list auto-loads → select a ticket → use status/assign actions"),
    ("banking", "open → transactions auto-load → use search → click a transaction to view details"),
    ("tasks", "open → task list auto-loads → use filter chips (all/active/completed) → click to edit"),
    ("project", "open → project list auto-loads → select a project → timeline/gantt renders"),
    ("mail", "open → unified inbox auto-loads → click a message → reply/compose actions"),
    ("calendar", "open → events load → click a day/event to open it"),
    ("meet", "open → join a room or list recordings"),
    ("minutes", "open → meetings/transcripts list → select one to view action items"),
    ("docs", "open → document list → open a doc into the editor"),
    ("sheet", "open → spreadsheet loads from drive params → write cells via formula bar"),
    ("drive", "open → tab bar (Bots/My Files/Shared/Public/Root) → navigate folders → open/select files"),
    ("database", "open → schema loads → pick a table → run queries"),
    ("social", "open → feed auto-loads → create a post via New Post"),
    ("hr", "open → employees list loads → switch tabs (Recruitment/Attendance/Performance)"),
    ("compliance", "open → checks load → switch tabs (Checks/Issues/Audit Log/Risks)"),
    ("kyc", "open → verifications list loads → select a verification"),
    ("tax", "open → documents load → filter NF-e/NFS-e/CT-e tabs"),
    ("analytics", "open → dashboards load → pick a report/dashboard"),
    ("monitoring", "open → health panels load → switch service/env filters"),
    ("attendant", "open → queue console loads → take/assign sessions"),
    ("campaigns", "open → campaigns list loads → filter views or open a campaign modal"),
    ("pos", "open → product grid renders → add to cart → checkout"),
    ("retail", "open → tabs load → filter by branch → view stock"),
    ("integrations", "open -> explore providers and action verbs -> inspect authentication and least-privilege guidance -> review connected services"),
    ("workspace", "open → pages list loads → open a page"),
    ("editor", "open → file tree → open a file"),
    ("vibe", "open → runs list → run/inspect an assistant run"),
    ("research", "open → search UI loads → run a web search"),
    ("video", "open → projects/cameras load → open a project"),
    ("learn", "open → courses load → open a course"),
    ("admin", "open → dashboard loads → manage users/roles/groups/billing"),
    ("settings", "open → settings panel loads → pick a section"),
];
/// UI automation hint for an app id (empty string when not declared).
pub fn ui_sequence_hint_for(app_id: &str) -> &'static str {
    UI_SEQUENCE_HINTS
        .iter()
        .find(|(id, _)| *id == app_id)
        .map(|(_, hint)| *hint)
        .unwrap_or("")
}
