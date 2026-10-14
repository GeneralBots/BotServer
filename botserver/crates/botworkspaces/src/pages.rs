use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breadcrumb {
    pub page_id: Uuid,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageBreadcrumbs {
    pub workspace_id: Uuid,
    pub workspace_name: String,
    pub workspace_icon: Option<WorkspaceIcon>,
    pub path: Vec<Breadcrumb>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageSummary {
    pub id: Uuid,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub parent_id: Option<Uuid>,
    pub has_children: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub last_edited_by: Uuid,
}

impl From<&Page> for PageSummary {
    fn from(page: &Page) -> Self {
        Self {
            id: page.id,
            title: page.title.clone(),
            icon: page.icon.clone(),
            parent_id: page.parent_id,
            has_children: !page.children.is_empty(),
            created_at: page.created_at,
            updated_at: page.updated_at,
            created_by: page.created_by,
            last_edited_by: page.last_edited_by,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagesCreatePageRequest {
    pub workspace_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub cover_image: Option<String>,
    pub template_id: Option<Uuid>,
    pub properties: Option<HashMap<String, PropertyValue>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagesUpdatePageRequest {
    pub title: Option<String>,
    pub icon: Option<WorkspaceIcon>,
    pub cover_image: Option<String>,
    pub properties: Option<HashMap<String, PropertyValue>>,
    pub permissions: Option<PagePermissions>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePageRequest {
    pub new_parent_id: Option<Uuid>,
    pub new_workspace_id: Option<Uuid>,
    pub position: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicatePageRequest {
    pub new_parent_id: Option<Uuid>,
    pub new_workspace_id: Option<Uuid>,
    pub include_children: bool,
    pub new_title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageExportOptions {
    pub format: ExportFormat,
    pub include_children: bool,
    pub include_images: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Markdown,
    Html,
    Pdf,
    PlainText,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageImportOptions {
    pub format: ImportFormat,
    pub parent_id: Option<Uuid>,
    pub workspace_id: Uuid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportFormat {
    Markdown,
    Html,
    Notion,
    Confluence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentPage {
    pub page_id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub workspace_name: String,
    pub accessed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FavoritePage {
    pub page_id: Uuid,
    pub workspace_id: Uuid,
    pub title: String,
    pub icon: Option<WorkspaceIcon>,
    pub added_at: DateTime<Utc>,
}

pub fn build_breadcrumbs(
    page_id: Uuid,
    pages: &HashMap<Uuid, Page>,
    workspace_name: &str,
    workspace_icon: Option<WorkspaceIcon>,
    workspace_id: Uuid,
) -> PageBreadcrumbs {
    let mut path = Vec::new();
    let mut current_id = Some(page_id);

    while let Some(id) = current_id {
        if let Some(page) = pages.get(&id) {
            path.push(Breadcrumb {
                page_id: page.id,
                title: page.title.clone(),
                icon: page.icon.clone(),
            });
            current_id = page.parent_id;
        } else {
            break;
        }
    }

    path.reverse();

    PageBreadcrumbs {
        workspace_id,
        workspace_name: workspace_name.to_string(),
        workspace_icon,
        path,
    }
}

pub fn get_page_depth(page_id: Uuid, pages: &HashMap<Uuid, Page>) -> usize {
    let mut depth = 0;
    let mut current_id = Some(page_id);

    while let Some(id) = current_id {
        if let Some(page) = pages.get(&id) {
            depth += 1;
            current_id = page.parent_id;
        } else {
            break;
        }
    }

    depth
}

pub fn get_all_descendants(page_id: Uuid, pages: &HashMap<Uuid, Page>) -> Vec<Uuid> {
    let mut descendants = Vec::new();

    if let Some(page) = pages.get(&page_id) {
        for child_id in &page.children {
            descendants.push(*child_id);
            descendants.extend(get_all_descendants(*child_id, pages));
        }
    }

    descendants
}

pub fn get_all_ancestors(page_id: Uuid, pages: &HashMap<Uuid, Page>) -> Vec<Uuid> {
    let mut ancestors = Vec::new();
    let mut current_id = pages.get(&page_id).and_then(|p| p.parent_id);

    while let Some(id) = current_id {
        ancestors.push(id);
        current_id = pages.get(&id).and_then(|p| p.parent_id);
    }

    ancestors
}

pub fn is_descendant_of(page_id: Uuid, potential_ancestor: Uuid, pages: &HashMap<Uuid, Page>) -> bool {
    let ancestors = get_all_ancestors(page_id, pages);
    ancestors.contains(&potential_ancestor)
}

pub fn can_move_page(
    page_id: Uuid,
    new_parent_id: Option<Uuid>,
    pages: &HashMap<Uuid, Page>,
) -> Result<(), String> {
    if let Some(new_pid) = new_parent_id {
        if page_id == new_pid {
            return Err("Cannot move page into itself".to_string());
        }

        if is_descendant_of(new_pid, page_id, pages) {
            return Err("Cannot move page into its own descendant".to_string());
        }
    }

    Ok(())
}

pub fn check_page_permission(
    page: &Page,
    user_id: Uuid,
    user_role: WorkspaceRole,
    required_permission: PagePermissionType,
) -> bool {
    if page.permissions.public {
        match required_permission {
            PagePermissionType::View => return true,
            PagePermissionType::Edit
                if page.permissions.public_edit => {
                    return true;
                }
            _ => {}
        }
    }

    if page.permissions.allowed_users.contains(&user_id) {
        return true;
    }

    if page.permissions.allowed_roles.contains(&user_role) {
        return true;
    }

    match user_role {
        WorkspaceRole::Owner | WorkspaceRole::Admin => true,
        WorkspaceRole::Editor => matches!(
            required_permission,
            PagePermissionType::View | PagePermissionType::Edit | PagePermissionType::Comment
        ),
        WorkspaceRole::Commenter => matches!(
            required_permission,
            PagePermissionType::View | PagePermissionType::Comment
        ),
        WorkspaceRole::Viewer => matches!(required_permission, PagePermissionType::View),
    }
}

