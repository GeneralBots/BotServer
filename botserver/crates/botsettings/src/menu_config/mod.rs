pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MenuItemType {
    App,
    Section,
    Divider,
    Link,
    SubMenu,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AppType {
    Docs,
    Paper,
    Forms,
    Sites,
    Drive,
    Sheet,
    Slides,
    Calendar,
    Meet,
    Tasks,
    Mail,
    Chat,
    Analytics,
    Monitoring,
    Settings,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuItem {
    pub id: String,
    pub item_type: MenuItemType,
    pub label: String,
    pub icon: Option<String>,
    pub route: Option<String>,
    pub app_type: Option<AppType>,
    pub children: Vec<MenuItem>,
    pub badge: Option<String>,
    pub tooltip: Option<String>,
    pub visible: bool,
    pub enabled: bool,
    pub order: i32,
    pub permissions: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuSection {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub items: Vec<MenuItem>,
    pub collapsed: bool,
    pub order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuConfig {
    pub sections: Vec<MenuSection>,
    pub footer_items: Vec<MenuItem>,
    pub user_menu_items: Vec<MenuItem>,
    pub quick_actions: Vec<MenuItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_menu_config() {
        let config = MenuConfig::default();
        assert!(!config.sections.is_empty());
        assert!(!config.footer_items.is_empty());
        assert!(!config.user_menu_items.is_empty());
    }

    #[test]
    fn test_library_section_exists() {
        let config = MenuConfig::default();
        let library = config.get_section_by_id("library");
        assert!(library.is_some());
        assert_eq!(library.map(|s| s.label.as_str()), Some("Library"));
    }

    #[test]
    fn test_docs_and_paper_in_library() {
        let config = MenuConfig::default();
        let library = config.get_section_by_id("library").unwrap();

        let has_docs = library.items.iter().any(|i| i.id == "docs");
        let has_paper = library.items.iter().any(|i| i.id == "paper");
        let has_forms = library.items.iter().any(|i| i.id == "forms");
        let has_sites = library.items.iter().any(|i| i.id == "sites");

        assert!(has_docs, "Library should contain Docs");
        assert!(has_paper, "Library should contain Paper");
        assert!(has_forms, "Library should contain Forms");
        assert!(has_sites, "Library should contain Sites");
    }

    #[test]
    fn test_app_type_properties() {
        assert_eq!(AppType::Docs.as_str(), "docs");
        assert_eq!(AppType::Paper.as_str(), "paper");
        assert_eq!(AppType::Docs.route(), "/docs");
        assert_eq!(AppType::Paper.route(), "/paper");
    }

    #[test]
    fn test_permission_filtering() {
        let config = MenuConfig::default();
        let user_permissions = vec!["member".to_string()];
        let filtered = config.filter_by_permissions(&user_permissions);

        for section in &filtered.sections {
            for item in &section.items {
                if !item.permissions.is_empty() {
                    assert!(item.permissions.iter().any(|p| user_permissions.contains(p)));
                }
            }
        }
    }

    #[test]
    fn test_json_serialization() {
        let config = MenuConfig::default();
        let json = config.to_json().unwrap();
        let deserialized = MenuConfig::from_json(&json).unwrap();
        assert_eq!(config.sections.len(), deserialized.sections.len());
    }

    #[test]
    fn test_docs_paper_clarification() {
        let clarification = get_docs_paper_clarification();
        assert!(clarification.contains_key("docs_purpose"));
        assert!(clarification.contains_key("paper_purpose"));
        assert!(clarification.contains_key("key_differences"));
    }

    #[test]
    fn test_library_section_info() {
        let info = get_library_section_info();
        assert_eq!(info.get("name"), Some(&"Library"));
        assert_eq!(info.get("previous_name"), Some(&"Sources"));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part5::*;
