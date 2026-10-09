use super::*;

impl Default for MenuConfig {
    fn default() -> Self {
        Self::new()
    }
}
impl MenuConfig {
    pub fn new() -> Self {
        let productivity_section = MenuSection::new("productivity", "Productivity", 1)
            .with_icon("⚡")
            .with_items(vec![
                MenuItem::app(AppType::Chat, 1),
                MenuItem::app(AppType::Mail, 2),
                MenuItem::app(AppType::Calendar, 3),
                MenuItem::app(AppType::Tasks, 4),
                MenuItem::app(AppType::Meet, 5),
            ]);

        let library_section = MenuSection::new("library", "Library", 2)
            .with_icon("📚")
            .with_items(vec![
                MenuItem::app(AppType::Drive, 1),
                MenuItem::app(AppType::Docs, 2)
                    .with_tooltip("Full-featured document editor with rich text formatting, collaboration, and export options"),
                MenuItem::app(AppType::Paper, 3)
                    .with_tooltip("Quick notes and lightweight text editor for fast capture"),
                MenuItem::app(AppType::Sheet, 4),
                MenuItem::app(AppType::Slides, 5),
                MenuItem::app(AppType::Forms, 6)
                    .with_tooltip("Create surveys, quizzes, and data collection forms"),
                MenuItem::app(AppType::Sites, 7)
                    .with_tooltip("Build and publish websites"),
            ]);

        let insights_section = MenuSection::new("insights", "Insights", 3)
            .with_icon("📊")
            .with_items(vec![
                MenuItem::app(AppType::Analytics, 1),
                MenuItem::app(AppType::Monitoring, 2),
            ]);

        let admin_section = MenuSection::new("admin", "Administration", 4)
            .with_icon("🛡️")
            .with_items(vec![
                MenuItem::app(AppType::Settings, 1),
                MenuItem::app(AppType::Admin, 2)
                    .with_permissions(vec!["admin", "owner"]),
            ]);

        let footer_items = vec![
            MenuItem {
                id: "help".to_string(),
                item_type: MenuItemType::Link,
                label: "Help & Support".to_string(),
                icon: Some("❓".to_string()),
                route: Some("/help".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 1,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem {
                id: "feedback".to_string(),
                item_type: MenuItemType::Link,
                label: "Send Feedback".to_string(),
                icon: Some("💭".to_string()),
                route: Some("/feedback".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 2,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
        ];

        let user_menu_items = vec![
            MenuItem {
                id: "profile".to_string(),
                item_type: MenuItemType::Link,
                label: "Profile".to_string(),
                icon: Some("👤".to_string()),
                route: Some("/profile".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 1,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem {
                id: "preferences".to_string(),
                item_type: MenuItemType::Link,
                label: "Preferences".to_string(),
                icon: Some("⚙️".to_string()),
                route: Some("/preferences".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 2,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem::divider(3),
            MenuItem {
                id: "logout".to_string(),
                item_type: MenuItemType::Link,
                label: "Sign Out".to_string(),
                icon: Some("🚪".to_string()),
                route: Some("/logout".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 4,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
        ];

        let quick_actions = vec![
            MenuItem {
                id: "new_doc".to_string(),
                item_type: MenuItemType::Link,
                label: "New Document".to_string(),
                icon: Some("📝".to_string()),
                route: Some("/docs/new".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: Some("Create a new full document".to_string()),
                visible: true,
                enabled: true,
                order: 1,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem {
                id: "new_note".to_string(),
                item_type: MenuItemType::Link,
                label: "New Note".to_string(),
                icon: Some("📓".to_string()),
                route: Some("/paper/new".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: Some("Create a quick note".to_string()),
                visible: true,
                enabled: true,
                order: 2,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem {
                id: "new_sheet".to_string(),
                item_type: MenuItemType::Link,
                label: "New Spreadsheet".to_string(),
                icon: Some("📊".to_string()),
                route: Some("/sheet/new".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 3,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem {
                id: "new_form".to_string(),
                item_type: MenuItemType::Link,
                label: "New Form".to_string(),
                icon: Some("📋".to_string()),
                route: Some("/forms/new".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 4,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem {
                id: "new_site".to_string(),
                item_type: MenuItemType::Link,
                label: "New Site".to_string(),
                icon: Some("🌐".to_string()),
                route: Some("/sites/new".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 5,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
            MenuItem {
                id: "schedule_meeting".to_string(),
                item_type: MenuItemType::Link,
                label: "Schedule Meeting".to_string(),
                icon: Some("📹".to_string()),
                route: Some("/meet/schedule".to_string()),
                app_type: None,
                children: Vec::new(),
                badge: None,
                tooltip: None,
                visible: true,
                enabled: true,
                order: 6,
                permissions: Vec::new(),
                metadata: HashMap::new(),
            },
        ];

        Self {
            sections: vec![
                productivity_section,
                library_section,
                insights_section,
                admin_section,
            ],
            footer_items,
            user_menu_items,
            quick_actions,
        }
    }

    pub fn get_all_items(&self) -> Vec<&MenuItem> {
        let mut items = Vec::new();

        for section in &self.sections {
            for item in &section.items {
                items.push(item);
                for child in &item.children {
                    items.push(child);
                }
            }
        }

        for item in &self.footer_items {
            items.push(item);
        }

        items
    }

    pub fn get_item_by_id(&self, id: &str) -> Option<&MenuItem> {
        self.get_all_items().into_iter().find(|item| item.id == id)
    }

    pub fn get_section_by_id(&self, id: &str) -> Option<&MenuSection> {
        self.sections.iter().find(|section| section.id == id)
    }

    pub fn filter_by_permissions(&self, user_permissions: &[String]) -> Self {
        let mut filtered = self.clone();

        for section in &mut filtered.sections {
            section.items.retain(|item| {
                if item.permissions.is_empty() {
                    return true;
                }
                item.permissions.iter().any(|p| user_permissions.contains(p))
            });
        }

        filtered.sections.retain(|section| !section.items.is_empty());

        filtered
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
