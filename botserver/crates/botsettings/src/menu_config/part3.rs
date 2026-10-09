use super::*;

impl MenuSection {
    pub fn new(id: &str, label: &str, order: i32) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            icon: None,
            items: Vec::new(),
            collapsed: false,
            order,
        }
    }

    pub fn with_icon(mut self, icon: &str) -> Self {
        self.icon = Some(icon.to_string());
        self
    }

    pub fn with_items(mut self, items: Vec<MenuItem>) -> Self {
        self.items = items;
        self
    }

    pub fn collapsed(mut self) -> Self {
        self.collapsed = true;
        self
    }
}
