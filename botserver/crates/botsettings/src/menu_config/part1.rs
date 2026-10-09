use super::*;

impl AppType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Docs => "docs",
            Self::Paper => "paper",
            Self::Forms => "forms",
            Self::Sites => "sites",
            Self::Drive => "drive",
            Self::Sheet => "sheet",
            Self::Slides => "slides",
            Self::Calendar => "calendar",
            Self::Meet => "meet",
            Self::Tasks => "tasks",
            Self::Mail => "mail",
            Self::Chat => "chat",
            Self::Analytics => "analytics",
            Self::Monitoring => "monitoring",
            Self::Settings => "settings",
            Self::Admin => "admin",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Docs => "Docs",
            Self::Paper => "Paper",
            Self::Forms => "Forms",
            Self::Sites => "Sites",
            Self::Drive => "Drive",
            Self::Sheet => "Sheet",
            Self::Slides => "Slides",
            Self::Calendar => "Calendar",
            Self::Meet => "Meet",
            Self::Tasks => "Tasks",
            Self::Mail => "Mail",
            Self::Chat => "Chat",
            Self::Analytics => "Analytics",
            Self::Monitoring => "Monitoring",
            Self::Settings => "Settings",
            Self::Admin => "Admin",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Docs => "📝",
            Self::Paper => "📓",
            Self::Forms => "📋",
            Self::Sites => "🌐",
            Self::Drive => "💾",
            Self::Sheet => "📊",
            Self::Slides => "📽️",
            Self::Calendar => "📅",
            Self::Meet => "📹",
            Self::Tasks => "✅",
            Self::Mail => "✉️",
            Self::Chat => "💬",
            Self::Analytics => "📈",
            Self::Monitoring => "🔍",
            Self::Settings => "⚙️",
            Self::Admin => "🛡️",
        }
    }

    pub fn route(&self) -> &'static str {
        match self {
            Self::Docs => "/docs",
            Self::Paper => "/paper",
            Self::Forms => "/forms",
            Self::Sites => "/sites",
            Self::Drive => "/drive",
            Self::Sheet => "/sheet",
            Self::Slides => "/slides",
            Self::Calendar => "/calendar",
            Self::Meet => "/meet",
            Self::Tasks => "/tasks",
            Self::Mail => "/mail",
            Self::Chat => "/chat",
            Self::Analytics => "/analytics",
            Self::Monitoring => "/monitoring",
            Self::Settings => "/settings",
            Self::Admin => "/admin",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Docs => "Full-featured document editor with rich text formatting",
            Self::Paper => "Quick notes and lightweight text editor",
            Self::Forms => "Create surveys, quizzes, and data collection forms",
            Self::Sites => "Build and publish websites",
            Self::Drive => "File storage and management",
            Self::Sheet => "Spreadsheets and data analysis",
            Self::Slides => "Presentations and slideshows",
            Self::Calendar => "Schedule and event management",
            Self::Meet => "Video conferencing and meetings",
            Self::Tasks => "Task and project management",
            Self::Mail => "Email client",
            Self::Chat => "Instant messaging and conversations",
            Self::Analytics => "Data analytics and reporting",
            Self::Monitoring => "System monitoring and health",
            Self::Settings => "Application settings",
            Self::Admin => "Administration panel",
        }
    }
}
