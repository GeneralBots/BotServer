use super::*;

impl std::fmt::Display for CleanupCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BotHistory => write!(f, "Bot History"),
            Self::ChatSessions => write!(f, "Chat Sessions"),
            Self::SystemLogs => write!(f, "System Logs"),
            Self::AccessLogs => write!(f, "Access Logs"),
            Self::DebugLogs => write!(f, "Debug Logs"),
            Self::TempFiles => write!(f, "Temp Files"),
            Self::CacheData => write!(f, "Cache Data"),
            Self::AnalyticsRaw => write!(f, "Analytics Raw Data"),
            Self::OldCalendarEvents => write!(f, "Old Calendar Events"),
            Self::Attachments => write!(f, "Old Attachments"),
        }
    }
}
impl CleanupCategory {
    pub fn all() -> Vec<Self> {
        vec![
            Self::BotHistory,
            Self::ChatSessions,
            Self::SystemLogs,
            Self::AccessLogs,
            Self::DebugLogs,
            Self::TempFiles,
            Self::CacheData,
            Self::AnalyticsRaw,
        ]
    }

    pub fn default_enabled() -> Vec<Self> {
        vec![
            Self::BotHistory,
            Self::ChatSessions,
            Self::SystemLogs,
            Self::DebugLogs,
            Self::TempFiles,
            Self::CacheData,
        ]
    }

    pub(super) fn table_name(&self) -> &'static str {
        match self {
            Self::BotHistory => "bot_conversation_history",
            Self::ChatSessions => "chat_sessions",
            Self::SystemLogs => "system_logs",
            Self::AccessLogs => "access_logs",
            Self::DebugLogs => "debug_logs",
            Self::TempFiles => "temp_files",
            Self::CacheData => "cache_entries",
            Self::AnalyticsRaw => "analytics_events",
            Self::OldCalendarEvents => "calendar_events",
            Self::Attachments => "attachments",
        }
    }

    pub(super) fn timestamp_column(&self) -> &'static str {
        match self {
            Self::BotHistory => "created_at",
            Self::ChatSessions => "last_activity_at",
            Self::SystemLogs => "logged_at",
            Self::AccessLogs => "accessed_at",
            Self::DebugLogs => "logged_at",
            Self::TempFiles => "created_at",
            Self::CacheData => "created_at",
            Self::AnalyticsRaw => "event_time",
            Self::OldCalendarEvents => "end_time",
            Self::Attachments => "created_at",
        }
    }

    pub(super) fn is_essential(&self) -> bool {
        matches!(self, Self::OldCalendarEvents | Self::Attachments)
    }
}
