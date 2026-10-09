use super::*;

impl MenuItem {
    pub fn app(app_type: AppType, order: i32) -> Self {
        Self {
            id: app_type.as_str().to_string(),
            item_type: MenuItemType::App,
            label: app_type.display_name().to_string(),
            icon: Some(app_type.icon().to_string()),
            route: Some(app_type.route().to_string()),
            app_type: Some(app_type),
            children: Vec::new(),
            badge: None,
            tooltip: None,
            visible: true,
            enabled: true,
            order,
            permissions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn section(id: &str, label: &str, order: i32) -> Self {
        Self {
            id: id.to_string(),
            item_type: MenuItemType::Section,
            label: label.to_string(),
            icon: None,
            route: None,
            app_type: None,
            children: Vec::new(),
            badge: None,
            tooltip: None,
            visible: true,
            enabled: true,
            order,
            permissions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn divider(order: i32) -> Self {
        Self {
            id: format!("divider_{order}"),
            item_type: MenuItemType::Divider,
            label: String::new(),
            icon: None,
            route: None,
            app_type: None,
            children: Vec::new(),
            badge: None,
            tooltip: None,
            visible: true,
            enabled: true,
            order,
            permissions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_children(mut self, children: Vec<MenuItem>) -> Self {
        self.children = children;
        self
    }

    pub fn with_badge(mut self, badge: &str) -> Self {
        self.badge = Some(badge.to_string());
        self
    }

    pub fn with_tooltip(mut self, tooltip: &str) -> Self {
        self.tooltip = Some(tooltip.to_string());
        self
    }

    pub fn with_permissions(mut self, permissions: Vec<&str>) -> Self {
        self.permissions = permissions.into_iter().map(String::from).collect();
        self
    }

    pub fn hidden(mut self) -> Self {
        self.visible = false;
        self
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}
