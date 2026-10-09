use super::*;

impl ApiUrls {
    pub const WHATSAPP_SEND: &'static str = "/api/whatsapp/send";

    // Files - JSON APIs
    pub const FILES_BY_ID: &'static str = "/api/files/{id}";

    // Messages - JSON APIs
    pub const MESSAGES: &'static str = "/api/messages";

    // Email Tracking - JSON APIs
    pub const EMAIL_TRACKING_LIST: &'static str = "/api/email/tracking/list";
    pub const EMAIL_TRACKING_STATS: &'static str = "/api/email/tracking/stats";

    // Instagram - JSON APIs
    pub const INSTAGRAM_WEBHOOK: &'static str = "/api/instagram/webhook";
    pub const INSTAGRAM_SEND: &'static str = "/api/instagram/send";

    // Monitoring - HTMX/HTML APIs
    pub const MONITORING_DASHBOARD: &'static str = "/api/ui/monitoring/dashboard";
    pub const MONITORING_SERVICES: &'static str = "/api/ui/monitoring/services";
    pub const MONITORING_RESOURCES: &'static str = "/api/ui/monitoring/resources";
    pub const MONITORING_LOGS: &'static str = "/api/ui/monitoring/logs";
    pub const MONITORING_LLM: &'static str = "/api/ui/monitoring/llm";
    pub const MONITORING_HEALTH: &'static str = "/api/ui/monitoring/health";
    pub const MONITORING_ALERTS: &'static str = "/api/monitoring/alerts";

    // Monitoring - Metrics & Widgets
    pub const MONITORING_TIMESTAMP: &'static str = "/api/ui/monitoring/timestamp";
    pub const MONITORING_BOTS: &'static str = "/api/ui/monitoring/bots";
    pub const MONITORING_SERVICES_STATUS: &'static str = "/api/ui/monitoring/services/status";
    pub const MONITORING_RESOURCES_BARS: &'static str = "/api/ui/monitoring/resources/bars";
    pub const MONITORING_ACTIVITY_LATEST: &'static str = "/api/ui/monitoring/activity/latest";
    pub const MONITORING_METRIC_SESSIONS: &'static str = "/api/ui/monitoring/metric/sessions";
    pub const MONITORING_METRIC_MESSAGES: &'static str = "/api/ui/monitoring/metric/messages";
    pub const MONITORING_METRIC_RESPONSE_TIME: &'static str =
        "/api/ui/monitoring/metric/response_time";
    pub const MONITORING_TREND_SESSIONS: &'static str = "/api/ui/monitoring/trend/sessions";
    pub const MONITORING_RATE_MESSAGES: &'static str = "/api/ui/monitoring/rate/messages";
    pub const MONITORING_SESSIONS_PANEL: &'static str = "/api/ui/monitoring/sessions";
    pub const MONITORING_MESSAGES_PANEL: &'static str = "/api/ui/monitoring/messages";

    // MS Teams - JSON APIs
    pub const MSTEAMS_MESSAGES: &'static str = "/api/msteams/messages";
    pub const MSTEAMS_SEND: &'static str = "/api/msteams/send";

    // Docs (Word Processor) - HTMX/HTML APIs
    pub const DOCS_NEW: &'static str = "/api/ui/docs/new";
    pub const DOCS_LIST: &'static str = "/api/ui/docs/list";
    pub const DOCS_SEARCH: &'static str = "/api/ui/docs/search";
    pub const DOCS_SAVE: &'static str = "/api/ui/docs/save";
    pub const DOCS_AUTOSAVE: &'static str = "/api/ui/docs/autosave";
    pub const DOCS_BY_ID: &'static str = "/api/ui/docs/{id}";
    pub const DOCS_DELETE: &'static str = "/api/ui/docs/{id}/delete";
    pub const DOCS_TEMPLATE_BLANK: &'static str = "/api/ui/docs/template/blank";
    pub const DOCS_TEMPLATE_MEETING: &'static str = "/api/ui/docs/template/meeting";
    pub const DOCS_TEMPLATE_REPORT: &'static str = "/api/ui/docs/template/report";
    pub const DOCS_TEMPLATE_LETTER: &'static str = "/api/ui/docs/template/letter";
    pub const DOCS_AI_SUMMARIZE: &'static str = "/api/ui/docs/ai/summarize";
    pub const DOCS_AI_EXPAND: &'static str = "/api/ui/docs/ai/expand";
    pub const DOCS_AI_IMPROVE: &'static str = "/api/ui/docs/ai/improve";
    pub const DOCS_AI_SIMPLIFY: &'static str = "/api/ui/docs/ai/simplify";
    pub const DOCS_AI_TRANSLATE: &'static str = "/api/ui/docs/ai/translate";
    pub const DOCS_AI_CUSTOM: &'static str = "/api/ui/docs/ai/custom";
    pub const DOCS_EXPORT_PDF: &'static str = "/api/ui/docs/export/pdf";
    pub const DOCS_EXPORT_DOCX: &'static str = "/api/ui/docs/export/docx";
    pub const DOCS_EXPORT_MD: &'static str = "/api/ui/docs/export/md";
    pub const DOCS_EXPORT_HTML: &'static str = "/api/ui/docs/export/html";
    pub const DOCS_EXPORT_TXT: &'static str = "/api/ui/docs/export/txt";
    pub const DOCS_WS: &'static str = "/ws/docs/{doc_id}";

    // Paper (Notes App) - HTMX/HTML APIs
    pub const PAPER_NEW: &'static str = "/api/ui/paper/new";
    pub const PAPER_LIST: &'static str = "/api/ui/paper/list";
    pub const PAPER_SEARCH: &'static str = "/api/ui/paper/search";
    pub const PAPER_SAVE: &'static str = "/api/ui/paper/save";
    pub const PAPER_AUTOSAVE: &'static str = "/api/ui/paper/autosave";
    pub const PAPER_BY_ID: &'static str = "/api/ui/paper/{id}";
    pub const PAPER_DELETE: &'static str = "/api/ui/paper/{id}/delete";
    pub const PAPER_TEMPLATE_BLANK: &'static str = "/api/ui/paper/template/blank";
    pub const PAPER_TEMPLATE_MEETING: &'static str = "/api/ui/paper/template/meeting";
    pub const PAPER_TEMPLATE_TODO: &'static str = "/api/ui/paper/template/todo";
    pub const PAPER_TEMPLATE_RESEARCH: &'static str = "/api/ui/paper/template/research";
    pub const PAPER_TEMPLATE_REPORT: &'static str = "/api/ui/paper/template/report";
    pub const PAPER_TEMPLATE_LETTER: &'static str = "/api/ui/paper/template/letter";
    pub const PAPER_AI_SUMMARIZE: &'static str = "/api/ui/paper/ai/summarize";
    pub const PAPER_AI_EXPAND: &'static str = "/api/ui/paper/ai/expand";
    pub const PAPER_AI_IMPROVE: &'static str = "/api/ui/paper/ai/improve";
    pub const PAPER_AI_SIMPLIFY: &'static str = "/api/ui/paper/ai/simplify";
    pub const PAPER_AI_TRANSLATE: &'static str = "/api/ui/paper/ai/translate";
    pub const PAPER_AI_CUSTOM: &'static str = "/api/ui/paper/ai/custom";
    pub const PAPER_EXPORT_PDF: &'static str = "/api/ui/paper/export/pdf";
    pub const PAPER_EXPORT_DOCX: &'static str = "/api/ui/paper/export/docx";
    pub const PAPER_EXPORT_MD: &'static str = "/api/ui/paper/export/md";
    pub const PAPER_EXPORT_HTML: &'static str = "/api/ui/paper/export/html";
    pub const PAPER_EXPORT_TXT: &'static str = "/api/ui/paper/export/txt";

    // Research - HTMX/HTML APIs
    pub const RESEARCH_COLLECTIONS: &'static str = "/api/ui/research/collections";
    pub const RESEARCH_COLLECTIONS_NEW: &'static str = "/api/ui/research/collections/new";
    pub const RESEARCH_COLLECTION_BY_ID: &'static str = "/api/ui/research/collections/{id}";
    pub const RESEARCH_SEARCH: &'static str = "/api/ui/research/search";
    pub const RESEARCH_RECENT: &'static str = "/api/ui/research/recent";
    pub const RESEARCH_TRENDING: &'static str = "/api/ui/research/trending";
    pub const RESEARCH_PROMPTS: &'static str = "/api/ui/research/prompts";
    pub const RESEARCH_WEB_SEARCH: &'static str = "/api/ui/research/web/search";
    pub const RESEARCH_WEB_SUMMARIZE: &'static str = "/api/ui/research/web/summarize";
    pub const RESEARCH_WEB_DEEP: &'static str = "/api/ui/research/web/deep";
    pub const RESEARCH_WEB_HISTORY: &'static str = "/api/ui/research/web/history";
    pub const RESEARCH_WEB_INSTANT: &'static str = "/api/ui/research/web/instant";
    pub const RESEARCH_EXPORT_CITATIONS: &'static str = "/api/ui/research/export/citations";

    // Sources - HTMX/HTML APIs
    pub const SOURCES_PROMPTS: &'static str = "/api/ui/sources/prompts";
    pub const SOURCES_TEMPLATES: &'static str = "/api/ui/sources/templates";
    pub const SOURCES_NEWS: &'static str = "/api/ui/sources/news";
    pub const SOURCES_MCP_SERVERS: &'static str = "/api/ui/sources/mcp-servers";
    pub const SOURCES_LLM_TOOLS: &'static str = "/api/ui/sources/llm-tools";
    pub const SOURCES_MODELS: &'static str = "/api/ui/sources/models";
    pub const SOURCES_SEARCH: &'static str = "/api/ui/sources/search";
    pub const SOURCES_REPOSITORIES: &'static str = "/api/ui/sources/repositories";
    pub const SOURCES_REPOSITORIES_CONNECT: &'static str = "/api/ui/sources/repositories/connect";
    pub const SOURCES_REPOSITORIES_DISCONNECT: &'static str =
        "/api/ui/sources/repositories/disconnect";
    pub const SOURCES_APPS: &'static str = "/api/ui/sources/apps";
    pub const SOURCES_MCP: &'static str = "/api/ui/sources/mcp";
    pub const SOURCES_MCP_BY_NAME: &'static str = "/api/ui/sources/mcp/{name}";
    pub const SOURCES_MCP_ENABLE: &'static str = "/api/ui/sources/mcp/{name}/enable";
    pub const SOURCES_MCP_DISABLE: &'static str = "/api/ui/sources/mcp/{name}/disable";
    pub const SOURCES_MCP_TOOLS: &'static str = "/api/ui/sources/mcp/{name}/tools";
    pub const SOURCES_MCP_TEST: &'static str = "/api/ui/sources/mcp/{name}/test";
    pub const SOURCES_MCP_SCAN: &'static str = "/api/ui/sources/mcp/scan";
    pub const SOURCES_MCP_EXAMPLES: &'static str = "/api/ui/sources/mcp/examples";
    pub const SOURCES_API_KEYS: &'static str = "/api/ui/sources/api-keys";
    pub const SOURCES_API_KEYS_BY_ID: &'static str = "/api/ui/sources/api-keys/{id}";
    pub const SOURCES_MENTIONS: &'static str = "/api/ui/sources/mentions";
    pub const SOURCES_TOOLS: &'static str = "/api/ui/sources/tools";

    // Sources Knowledge Base - HTMX/HTML APIs
    pub const SOURCES_KB_UPLOAD: &'static str = "/api/ui/sources/kb/upload";
    pub const SOURCES_KB_LIST: &'static str = "/api/ui/sources/kb/list";
    pub const SOURCES_KB_QUERY: &'static str = "/api/ui/sources/kb/query";
    pub const SOURCES_KB_BY_ID: &'static str = "/api/ui/sources/kb/{id}";
    pub const SOURCES_KB_REINDEX: &'static str = "/api/ui/sources/kb/reindex";
    pub const SOURCES_KB_STATS: &'static str = "/api/ui/sources/kb/stats";

    // Workspaces - JSON APIs
    pub const WORKSPACES: &'static str = "/api/workspaces";
    pub const WORKSPACE_BY_ID: &'static str = "/api/workspaces/{workspace_id}";
    pub const WORKSPACE_PAGES: &'static str = "/api/workspaces/{workspace_id}/pages";
    pub const WORKSPACE_MEMBERS: &'static str = "/api/workspaces/{workspace_id}/members";
    pub const WORKSPACE_MEMBER: &'static str = "/api/workspaces/{workspace_id}/members/{user_id}";
    pub const WORKSPACE_SEARCH: &'static str = "/api/workspaces/{workspace_id}/search";
    pub const WORKSPACE_COMMANDS: &'static str = "/api/workspaces/commands";
    pub const PAGE_BY_ID: &'static str = "/api/pages/{page_id}";

    // Project - JSON APIs
    pub const PROJECTS: &'static str = "/projects";
    pub const PROJECT_BY_ID: &'static str = "/projects/{project_id}";
    pub const PROJECT_TASKS: &'static str = "/projects/{project_id}/tasks";
    pub const PROJECT_GANTT: &'static str = "/projects/{project_id}/gantt";
    pub const PROJECT_TIMELINE: &'static str = "/projects/{project_id}/timeline";
    pub const PROJECT_CRITICAL_PATH: &'static str = "/projects/{project_id}/critical-path";
    pub const PROJECT_TASK_PROGRESS: &'static str = "/tasks/{task_id}/progress";
    pub const PROJECT_TASK_DEPENDENCIES: &'static str = "/tasks/{task_id}/dependencies";
    pub const PROJECT_TASK: &'static str = "/tasks/{task_id}";

    // Goals (OKR) - JSON APIs
    pub const GOALS_OBJECTIVES: &'static str = "/api/goals/objectives";
    pub const GOALS_OBJECTIVE_BY_ID: &'static str = "/api/goals/objectives/{id}";
    pub const GOALS_KEY_RESULTS: &'static str = "/api/goals/objectives/{id}/key-results";
    pub const GOALS_KEY_RESULT_BY_ID: &'static str = "/api/goals/key-results/{id}";
    pub const GOALS_CHECK_IN: &'static str = "/api/goals/key-results/{id}/check-in";
    pub const GOALS_HISTORY: &'static str = "/api/goals/key-results/{id}/history";
    pub const GOALS_DASHBOARD: &'static str = "/api/goals/dashboard";
    pub const GOALS_ALIGNMENT: &'static str = "/api/goals/alignment";
    pub const GOALS_AI_SUGGEST: &'static str = "/api/goals/ai/suggest";

    // Security Admin - JSON APIs
    pub const SECURITY_OVERVIEW: &'static str = "/api/security/overview";
    pub const SECURITY_SCAN: &'static str = "/api/security/scan";
    pub const SECURITY_TLS: &'static str = "/api/security/tls";
    pub const SECURITY_RATE_LIMIT: &'static str = "/api/security/rate-limit";
    pub const SECURITY_CORS: &'static str = "/api/security/cors";
    pub const SECURITY_AUDIT: &'static str = "/api/security/audit";
    pub const SECURITY_API_KEYS: &'static str = "/api/security/api-keys";
    pub const SECURITY_API_KEY_BY_ID: &'static str = "/api/security/api-keys/{key_id}";
    pub const SECURITY_MFA: &'static str = "/api/security/mfa";
    pub const SECURITY_SESSIONS: &'static str = "/api/security/sessions";
    pub const SECURITY_SESSION_BY_ID: &'static str = "/api/security/sessions/{session_id}";
    pub const SECURITY_USER_SESSIONS: &'static str = "/api/security/users/{user_id}/sessions";
    pub const SECURITY_PASSWORD_POLICY: &'static str = "/api/security/password-policy";

    // Player - JSON APIs
    pub const PLAYER_FILE: &'static str = "/api/player/{bot_id}/file/*path";
    pub const PLAYER_STREAM: &'static str = "/api/player/{bot_id}/stream/*path";
    pub const PLAYER_THUMBNAIL: &'static str = "/api/player/{bot_id}/thumbnail/*path";

    // Canvas - JSON APIs
    pub const CANVAS_LIST: &'static str = "/api/canvas";
    pub const CANVAS_BY_ID: &'static str = "/api/canvas/{id}";
    pub const CANVAS_ELEMENTS: &'static str = "/api/canvas/{id}/elements";
    pub const CANVAS_ELEMENT_BY_ID: &'static str = "/api/canvas/{id}/elements/{element_id}";
    pub const CANVAS_EXPORT: &'static str = "/api/canvas/{id}/export";
    pub const CANVAS_COLLABORATE: &'static str = "/api/canvas/{id}/collaborate";

    // WebSocket endpoints
    pub const WS: &'static str = "/ws";
    pub const WS_MEET: &'static str = "/ws/meet";
    pub const WS_CHAT: &'static str = "/ws/chat";
    pub const WS_NOTIFICATIONS: &'static str = "/ws/notifications";
    pub const WS_ATTENDANT: &'static str = "/ws/attendant";

    // Terminal endpoints
    pub const TERMINAL_WS: &'static str = "/api/terminal/ws";
    pub const TERMINAL_LIST: &'static str = "/api/terminal/list";
    pub const TERMINAL_CREATE: &'static str = "/api/terminal/create";
    pub const TERMINAL_KILL: &'static str = "/api/terminal/kill";
}
