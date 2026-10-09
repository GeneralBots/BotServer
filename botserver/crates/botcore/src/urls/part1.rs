use super::*;

impl ApiUrls {
    pub const USERS: &'static str = "/api/users";
    pub const USER_BY_ID: &'static str = "/api/users/{id}";
    pub const USER_LOGIN: &'static str = "/api/users/login";
    pub const USER_LOGOUT: &'static str = "/api/users/logout";
    pub const USER_REGISTER: &'static str = "/api/users/register";
    pub const USER_PROFILE: &'static str = "/api/users/profile";
    pub const USER_PASSWORD: &'static str = "/api/users/password";
    pub const USER_SETTINGS: &'static str = "/api/users/settings";
    pub const USER_PROVISION: &'static str = "/api/users/provision";
    pub const USER_DEPROVISION: &'static str = "/api/users/{id}/deprovision";

    // Groups - JSON APIs
    pub const GROUPS: &'static str = "/api/groups";
    pub const GROUP_BY_ID: &'static str = "/api/groups/{id}";
    pub const GROUP_MEMBERS: &'static str = "/api/groups/{id}/members";
    pub const GROUP_ADD_MEMBER: &'static str = "/api/groups/{id}/members/{user_id}";
    pub const GROUP_REMOVE_MEMBER: &'static str = "/api/groups/{id}/members/{user_id}";
    pub const GROUP_PERMISSIONS: &'static str = "/api/groups/{id}/permissions";

    // Product - JSON APIs
    pub const PRODUCT: &'static str = "/api/product";

    // Auth - JSON APIs
    pub const AUTH: &'static str = "/api/auth";
    pub const AUTH_TOKEN: &'static str = "/api/auth/token";
    pub const AUTH_REFRESH: &'static str = "/api/auth/refresh";
    pub const AUTH_VERIFY: &'static str = "/api/auth/verify";
    pub const AUTH_OAUTH: &'static str = "/api/auth/oauth";
    pub const AUTH_OAUTH_CALLBACK: &'static str = "/api/auth/oauth/callback";

    // Sessions - JSON APIs
    pub const SESSIONS: &'static str = "/api/sessions";
    pub const SESSION_BY_ID: &'static str = "/api/sessions/{id}";
    pub const SESSION_HISTORY: &'static str = "/api/sessions/{id}/history";
    pub const SESSION_START: &'static str = "/api/sessions/{id}/start";
    pub const SESSION_END: &'static str = "/api/sessions/{id}/end";

    // Bots - JSON APIs
    pub const BOTS: &'static str = "/api/bots";
    pub const BOT_BY_ID: &'static str = "/api/bots/{id}";
    pub const BOT_CONFIG: &'static str = "/api/bots/{id}/config";
    pub const BOT_DEPLOY: &'static str = "/api/bots/{id}/deploy";
    pub const BOT_LOGS: &'static str = "/api/bots/{id}/logs";
    pub const BOT_METRICS: &'static str = "/api/bots/{id}/metrics";

    // Drive / Files - JSON APIs (actual routes match /api/files/*)
    pub const FILES_LIST: &'static str = "/api/files/list";
    pub const FILES_BUCKETS: &'static str = "/api/files/buckets";
    pub const FILES_QUOTA: &'static str = "/api/files/quota";
    pub const FILES_RECENT: &'static str = "/api/files/recent";
    pub const FILES_SEARCH: &'static str = "/api/files/search";
    pub const FILES_WRITE: &'static str = "/api/files/write";
    pub const FILES_DOWNLOAD: &'static str = "/api/files/download";
    pub const FILES_DOWNLOAD_BINARY: &'static str = "/api/files/download-binary";
    pub const FILES_DELETE: &'static str = "/api/files/delete";
    pub const FILES_CREATE_FOLDER: &'static str = "/api/files/createFolder";
    pub const FILES_COPY: &'static str = "/api/files/copy";
    pub const FILES_MOVE: &'static str = "/api/files/move";
    pub const FILES_OPEN: &'static str = "/api/files/open";
    pub const FILES_AI_CHAT: &'static str = "/api/files/ai/chat";
    pub const FILES_FAVORITE: &'static str = "/api/files/favorite";
    pub const FILES_SHARED: &'static str = "/api/files/shared";
    // Legacy aliases (deprecated, use FILES_* instead)
    pub const DRIVE_LIST: &'static str = "/api/files/list";
    pub const DRIVE_UPLOAD: &'static str = "/api/files/write";
    pub const DRIVE_DOWNLOAD: &'static str = "/api/files/download";
    pub const DRIVE_DELETE: &'static str = "/api/files/delete";
    pub const DRIVE_MKDIR: &'static str = "/api/files/createFolder";
    pub const DRIVE_MOVE: &'static str = "/api/files/move";
    pub const DRIVE_COPY: &'static str = "/api/files/copy";
    pub const DRIVE_SHARE: &'static str = "/api/files/shared";

    // Email - JSON APIs
    pub const EMAIL_ACCOUNTS: &'static str = "/api/email/accounts";
    pub const EMAIL_ACCOUNT_BY_ID: &'static str = "/api/email/accounts/{id}";
    pub const EMAIL_LIST: &'static str = "/api/email/list";
    pub const EMAIL_SEND: &'static str = "/api/email/send";
    pub const EMAIL_DRAFT: &'static str = "/api/email/draft";
    pub const EMAIL_FOLDERS: &'static str = "/api/email/folders/{account_id}";
    pub const EMAIL_LATEST: &'static str = "/api/email/latest";
    pub const EMAIL_GET: &'static str = "/api/email/get/{campaign_id}";
    pub const EMAIL_CLICK: &'static str = "/api/email/click/{campaign_id}/{email}";

    // Email - HTMX/HTML APIs
    pub const EMAIL_ACCOUNTS_HTMX: &'static str = "/api/ui/email/accounts";
    pub const EMAIL_LIST_HTMX: &'static str = "/api/ui/email/list";
    pub const EMAIL_FOLDERS_HTMX: &'static str = "/api/ui/email/folders/{account_id}";
    pub const EMAIL_COMPOSE_HTMX: &'static str = "/api/ui/email/compose";
    pub const EMAIL_CONTENT_HTMX: &'static str = "/api/ui/email/content/{id}";
    pub const EMAIL_LABELS_HTMX: &'static str = "/api/ui/email/labels";
    pub const EMAIL_TEMPLATES_HTMX: &'static str = "/api/ui/email/templates";
    pub const EMAIL_SIGNATURES_HTMX: &'static str = "/api/ui/email/signatures";
    pub const EMAIL_RULES_HTMX: &'static str = "/api/ui/email/rules";
    pub const EMAIL_SEARCH_HTMX: &'static str = "/api/ui/email/search";
    pub const EMAIL_AUTO_RESPONDER_HTMX: &'static str = "/api/ui/email/auto-responder";

    // Calendar - JSON APIs
    pub const CALENDAR_EVENTS: &'static str = "/api/calendar/events";
    pub const CALENDAR_EVENT_BY_ID: &'static str = "/api/calendar/events/{id}";
    pub const CALENDAR_REMINDERS: &'static str = "/api/calendar/reminders";
    pub const CALENDAR_SHARE: &'static str = "/api/calendar/share";
    pub const CALENDAR_SYNC: &'static str = "/api/calendar/sync";
    pub const CALENDAR_EXPORT: &'static str = "/api/calendar/export.ics";
    pub const CALENDAR_IMPORT: &'static str = "/api/calendar/import";
    pub const CALENDAR_CALENDARS_JSON: &'static str = "/api/calendar/calendars";
    pub const CALENDAR_UPCOMING_JSON: &'static str = "/api/calendar/events/upcoming";

    // Calendar - HTMX/HTML APIs
    pub const CALENDAR_CALENDARS: &'static str = "/api/ui/calendar/calendars";
    pub const CALENDAR_UPCOMING: &'static str = "/api/ui/calendar/events/upcoming";
    pub const CALENDAR_NEW_EVENT_FORM: &'static str = "/api/ui/calendar/events/new";
    pub const CALENDAR_NEW_CALENDAR_FORM: &'static str = "/api/ui/calendar/calendars/new";

    // Tasks - JSON APIs
    pub const TASKS: &'static str = "/api/tasks";
    pub const TASK_BY_ID: &'static str = "/api/tasks/{id}";
    pub const TASK_ASSIGN: &'static str = "/api/tasks/{id}/assign";
    pub const TASK_STATUS: &'static str = "/api/tasks/{id}/status";
    pub const TASK_PRIORITY: &'static str = "/api/tasks/{id}/priority";
    pub const TASK_COMMENTS: &'static str = "/api/tasks/{id}/comments";
    pub const TASKS_STATS_JSON: &'static str = "/api/tasks/stats/json";

    // Tasks - HTMX/HTML APIs
    pub const TASKS_LIST_HTMX: &'static str = "/api/ui/tasks";
    pub const TASKS_GET_HTMX: &'static str = "/api/ui/tasks/{id}";
    pub const TASKS_STATS: &'static str = "/api/ui/tasks/stats";
    pub const TASKS_COMPLETED: &'static str = "/api/ui/tasks/completed";
    pub const TASKS_TIME_SAVED: &'static str = "/api/ui/tasks/time-saved";

    // Meet - JSON APIs
    pub const MEET_CREATE: &'static str = "/api/meet/create";
    pub const MEET_ROOMS: &'static str = "/api/meet/rooms";
    pub const MEET_ROOM_BY_ID: &'static str = "/api/meet/rooms/{id}";
    pub const MEET_JOIN: &'static str = "/api/meet/rooms/{id}/join";
    pub const MEET_LEAVE: &'static str = "/api/meet/rooms/{id}/leave";
    pub const MEET_TOKEN: &'static str = "/api/meet/token";
    pub const MEET_INVITE: &'static str = "/api/meet/invite";
    pub const MEET_TRANSCRIPTION: &'static str = "/api/meet/rooms/{id}/transcription";
    pub const MEET_RECORDING_START: &'static str = "/api/meet/rooms/{id}/recording/start";
    pub const MEET_RECORDING_STOP: &'static str = "/api/meet/rooms/{id}/recording/stop";
    pub const MEET_RECORDING_PAUSE: &'static str = "/api/meet/rooms/{id}/recording/{recording_id}/pause";
    pub const MEET_RECORDING_RESUME: &'static str = "/api/meet/rooms/{id}/recording/{recording_id}/resume";
    pub const MEET_RECORDINGS_LIST: &'static str = "/api/meet/rooms/{id}/recordings";
    pub const MEET_RECORDING_GET: &'static str = "/api/meet/recordings/{recording_id}";
    pub const MEET_RECORDING_FILE: &'static str = "/api/meet/recordings/{recording_id}/file";
    pub const MEET_RECORDING_DELETE: &'static str = "/api/meet/recordings/{recording_id}";
    pub const MEET_TURN_CREDENTIALS: &'static str = "/api/meet/turn-credentials";
    pub const MEET_SCHEDULE: &'static str = "/api/meet/schedule";
    pub const MEET_DASHBOARD_STATS: &'static str = "/api/meet/dashboard/stats";
    pub const WS_MEET_SIGNALING: &'static str = "/ws/meet/{room_id}";

    // Meet - HTMX/HTML APIs
    pub const MEET_ROOMS_HTMX: &'static str = "/api/ui/meet/rooms";
    pub const MEET_PARTICIPANTS_HTMX: &'static str = "/api/ui/meet/participants";
    pub const MEET_RECENT_HTMX: &'static str = "/api/ui/meet/recent";
    pub const MEET_SCHEDULED_HTMX: &'static str = "/api/ui/meet/scheduled";
    pub const MEET_DASHBOARD_LIST_HTMX: &'static str = "/api/ui/meet/dashboard/list";
    pub const MEET_HTMX_DASHBOARD_STATS: &'static str = "/api/meet/htmx/dashboard/stats";
    pub const MEET_HTMX_DASHBOARD_LIST: &'static str = "/api/meet/htmx/dashboard/list";
    pub const MEET_HTMX_ROOM_CREATE: &'static str = "/api/meet/htmx/rooms/create";

    // Voice - JSON APIs
    pub const VOICE_START: &'static str = "/api/voice/start";
    pub const VOICE_STOP: &'static str = "/api/voice/stop";
    pub const VOICE_STATUS: &'static str = "/api/voice/status";

    // DNS - JSON APIs
    pub const DNS_REGISTER: &'static str = "/api/dns/register";
    pub const DNS_REMOVE: &'static str = "/api/dns/remove";
    pub const DNS_LIST: &'static str = "/api/dns/list";
    pub const DNS_UPDATE: &'static str = "/api/dns/update";

    // Analytics - JSON APIs
    pub const ANALYTICS_DASHBOARD: &'static str = "/api/analytics/dashboard";
    pub const ANALYTICS_METRIC: &'static str = "/api/analytics/metric";
    pub const METRICS: &'static str = "/api/metrics";

    // Analytics - HTMX/HTML APIs
    pub const ANALYTICS_MESSAGES_COUNT: &'static str = "/api/ui/analytics/messages/count";
    pub const ANALYTICS_SESSIONS_ACTIVE: &'static str = "/api/ui/analytics/sessions/active";
    pub const ANALYTICS_RESPONSE_AVG: &'static str = "/api/ui/analytics/response/avg";
    pub const ANALYTICS_LLM_TOKENS: &'static str = "/api/ui/analytics/llm/tokens";
    pub const ANALYTICS_STORAGE_USAGE: &'static str = "/api/ui/analytics/storage/usage";
    pub const ANALYTICS_ERRORS_COUNT: &'static str = "/api/ui/analytics/errors/count";
    pub const ANALYTICS_TIMESERIES_MESSAGES: &'static str = "/api/ui/analytics/timeseries/messages";
    pub const ANALYTICS_TIMESERIES_RESPONSE: &'static str =
        "/api/ui/analytics/timeseries/response_time";
    pub const ANALYTICS_CHANNELS_DISTRIBUTION: &'static str =
        "/api/ui/analytics/channels/distribution";
    pub const ANALYTICS_BOTS_PERFORMANCE: &'static str = "/api/ui/analytics/bots/performance";
    pub const ANALYTICS_ACTIVITY_RECENT: &'static str = "/api/ui/analytics/activity/recent";
    pub const ANALYTICS_QUERIES_TOP: &'static str = "/api/ui/analytics/queries/top";
    pub const ANALYTICS_CHAT: &'static str = "/api/ui/analytics/chat";
    pub const ANALYTICS_LLM_STATS: &'static str = "/api/ui/analytics/llm/stats";
    pub const ANALYTICS_BUDGET_STATUS: &'static str = "/api/ui/analytics/budget/status";

    // Admin - JSON APIs
    pub const ADMIN_DASHBOARD: &'static str = "/api/admin/dashboard";
    pub const ADMIN_STATS: &'static str = "/api/admin/stats";
    pub const ADMIN_STATS_USERS: &'static str = "/api/admin/stats/users";
    pub const ADMIN_STATS_GROUPS: &'static str = "/api/admin/stats/groups";
    pub const ADMIN_STATS_BOTS: &'static str = "/api/admin/stats/bots";
    pub const ADMIN_STATS_STORAGE: &'static str = "/api/admin/stats/storage";
    pub const ADMIN_USERS: &'static str = "/api/admin/users";
    pub const ADMIN_GROUPS: &'static str = "/api/admin/groups";
    pub const ADMIN_BOTS: &'static str = "/api/admin/bots";
    pub const ADMIN_DNS: &'static str = "/api/admin/dns";
    pub const ADMIN_BILLING: &'static str = "/api/admin/billing";
    pub const ADMIN_SYSTEM: &'static str = "/api/admin/system";
    pub const ADMIN_LOGS: &'static str = "/api/admin/logs";
    pub const ADMIN_BACKUPS: &'static str = "/api/admin/backups";
    pub const ADMIN_SERVICES: &'static str = "/api/admin/services";
    pub const ADMIN_AUDIT: &'static str = "/api/admin/audit";

    // Health/Status - JSON APIs
    pub const HEALTH: &'static str = "/api/health";
    pub const STATUS: &'static str = "/api/status";
    pub const SERVICES_STATUS: &'static str = "/api/services/status";

    // i18n - JSON APIs
    pub const I18N_TRANSLATIONS: &'static str = "/api/i18n/{locale}";
    pub const I18N_LOCALES: &'static str = "/api/i18n/locales";

    // Knowledge Base - JSON APIs
    pub const KB_SEARCH: &'static str = "/api/kb/search";
    pub const KB_UPLOAD: &'static str = "/api/kb/upload";
    pub const KB_DOCUMENTS: &'static str = "/api/kb/documents";
    pub const KB_DOCUMENT_BY_ID: &'static str = "/api/kb/documents/{id}";
    pub const KB_INDEX: &'static str = "/api/kb/index";
    pub const KB_EMBEDDINGS: &'static str = "/api/kb/embeddings";

    // LLM - JSON APIs
    pub const LLM_CHAT: &'static str = "/api/llm/chat";
    pub const LLM_COMPLETIONS: &'static str = "/api/llm/completions";
    pub const LLM_EMBEDDINGS: &'static str = "/api/llm/embeddings";
    pub const LLM_MODELS: &'static str = "/api/llm/models";
    pub const LLM_GENERATE: &'static str = "/api/llm/generate";
    pub const LLM_IMAGE: &'static str = "/api/llm/image";

    // Attendance - JSON APIs
    pub const ATTENDANCE_QUEUE: &'static str = "/api/attendance/queue";
    pub const ATTENDANCE_ATTENDANTS: &'static str = "/api/attendance/attendants";
    pub const ATTENDANCE_ASSIGN: &'static str = "/api/attendance/assign";
    pub const ATTENDANCE_TRANSFER: &'static str = "/api/attendance/transfer";
    pub const ATTENDANCE_RESOLVE: &'static str = "/api/attendance/resolve/{session_id}";
    pub const ATTENDANCE_INSIGHTS: &'static str = "/api/attendance/insights";
    pub const ATTENDANCE_RESPOND: &'static str = "/api/attendance/respond";
    pub const ATTENDANCE_KANBAN: &'static str = "/api/attendance/kanban";
    pub const ATTENDANCE_ASSIGN_BY_SKILL: &'static str = "/api/attendance/assign/by-skill";
    pub const ATTENDANCE_LLM_TIPS: &'static str = "/api/attendance/llm/tips";
    pub const ATTENDANCE_LLM_POLISH: &'static str = "/api/attendance/llm/polish";
    pub const ATTENDANCE_LLM_SMART_REPLIES: &'static str = "/api/attendance/llm/smart-replies";
    pub const ATTENDANCE_LLM_SUMMARY: &'static str = "/api/attendance/llm/summary/{session_id}";
    pub const ATTENDANCE_LLM_SENTIMENT: &'static str = "/api/attendance/llm/sentiment";
    pub const ATTENDANCE_LLM_CONFIG: &'static str = "/api/attendance/llm/config/{bot_id}";

    // AutoTask - JSON APIs
    pub const AUTOTASK_CREATE: &'static str = "/api/autotask/create";
    pub const AUTOTASK_CLASSIFY: &'static str = "/api/autotask/classify";
    pub const AUTOTASK_COMPILE: &'static str = "/api/autotask/compile";
    pub const AUTOTASK_EXECUTE: &'static str = "/api/autotask/execute";
    pub const AUTOTASK_SIMULATE: &'static str = "/api/autotask/simulate/{plan_id}";
    pub const AUTOTASK_GET: &'static str = "/api/autotask/tasks/{task_id}";
    pub const AUTOTASK_STATS: &'static str = "/api/autotask/stats";
    pub const AUTOTASK_PAUSE: &'static str = "/api/autotask/{task_id}/pause";
    pub const AUTOTASK_RESUME: &'static str = "/api/autotask/{task_id}/resume";
    pub const AUTOTASK_CANCEL: &'static str = "/api/autotask/{task_id}/cancel";
    pub const AUTOTASK_TASK_SIMULATE: &'static str = "/api/autotask/{task_id}/simulate";
    pub const AUTOTASK_DECISIONS: &'static str = "/api/autotask/{task_id}/decisions";
    pub const AUTOTASK_DECIDE: &'static str = "/api/autotask/{task_id}/decide";
    pub const AUTOTASK_APPROVALS: &'static str = "/api/autotask/{task_id}/approvals";
    pub const AUTOTASK_APPROVE: &'static str = "/api/autotask/{task_id}/approve";
    pub const AUTOTASK_TASK_EXECUTE: &'static str = "/api/autotask/{task_id}/execute";
    pub const AUTOTASK_LOGS: &'static str = "/api/autotask/{task_id}/logs";
    pub const AUTOTASK_RECOMMENDATIONS_APPLY: &'static str =
        "/api/autotask/recommendations/{rec_id}/apply";
    pub const AUTOTASK_PENDING: &'static str = "/api/autotask/pending";
    pub const AUTOTASK_PENDING_ITEM: &'static str = "/api/autotask/pending/{item_id}";

    // AutoTask - HTMX/HTML APIs
    pub const AUTOTASK_LIST: &'static str = "/api/ui/autotask/list";

    // DB - JSON APIs
    pub const DB_TABLE: &'static str = "/api/db/{table}";
    pub const DB_TABLE_RECORD: &'static str = "/api/db/{table}/{id}";
    pub const DB_TABLE_COUNT: &'static str = "/api/db/{table}/count";
    pub const DB_TABLE_SEARCH: &'static str = "/api/db/{table}/search";

    // Designer - HTMX/HTML APIs
    pub const DESIGNER_FILES: &'static str = "/api/ui/designer/files";
    pub const DESIGNER_LOAD: &'static str = "/api/ui/designer/load";
    pub const DESIGNER_SAVE: &'static str = "/api/ui/designer/save";
    pub const DESIGNER_VALIDATE: &'static str = "/api/ui/designer/validate";
    pub const DESIGNER_EXPORT: &'static str = "/api/ui/designer/export";
    pub const DESIGNER_MODIFY: &'static str = "/api/ui/designer/modify";
    pub const DESIGNER_DIALOGS: &'static str = "/api/ui/designer/dialogs";
    pub const DESIGNER_DIALOG_BY_ID: &'static str = "/api/ui/designer/dialogs/{id}";

    // Mail/WhatsApp - JSON APIs
    pub const MAIL_SEND: &'static str = "/api/mail/send";
}
