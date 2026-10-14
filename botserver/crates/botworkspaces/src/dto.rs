use super::*;

pub(crate) fn db_to_workspace(db: DbWorkspace, members: Vec<WorkspaceMember>, root_pages: Vec<Uuid>) -> Workspace {
    let icon = match (&db.icon_type, &db.icon_value) {
        (Some(t), Some(v)) => Some(WorkspaceIcon {
            icon_type: IconType::from_label(t),
            value: v.clone(),
        }),
        _ => None,
    };
    let settings: WorkspaceSettings = serde_json::from_value(db.settings).unwrap_or_default();

    Workspace {
        id: db.id,
        branch_id: db.branch_id,
        name: db.name,
        description: db.description,
        icon,
        cover_image: db.cover_image,
        settings,
        created_by: db.created_by,
        created_at: db.created_at,
        updated_at: db.updated_at,
        members,
        root_pages,
    }
}

pub(crate) fn db_to_page(db: DbWorkspacePage, children: Vec<Uuid>) -> Page {
    let icon = match (&db.icon_type, &db.icon_value) {
        (Some(t), Some(v)) => Some(WorkspaceIcon {
            icon_type: IconType::from_label(t),
            value: v.clone(),
        }),
        _ => None,
    };

    let blocks: Vec<Block> = serde_json::from_value(db.content).unwrap_or_default();
    let properties: HashMap<String, PropertyValue> =
        serde_json::from_value(db.properties).unwrap_or_default();

    Page {
        id: db.id,
        workspace_id: db.workspace_id,
        parent_id: db.parent_id,
        title: db.title,
        icon,
        cover_image: db.cover_image,
        blocks,
        children,
        properties,
        permissions: PagePermissions {
            inherit_from_parent: true,
            public: db.is_public,
            public_edit: db.public_edit,
            allowed_users: vec![],
            allowed_roles: vec![],
        },
        is_template: db.is_template,
        template_id: db.template_id,
        created_at: db.created_at,
        updated_at: db.updated_at,
        created_by: db.created_by,
        last_edited_by: db.last_edited_by.unwrap_or(db.created_by),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateWorkspaceRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateWorkspaceRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<WorkspaceIcon>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePageRequest {
    pub title: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePageRequest {
    pub title: Option<String>,
    pub icon: Option<WorkspaceIcon>,
    pub blocks: Option<Vec<Block>>,
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub user_id: Uuid,
    pub role: WorkspaceRole,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub content: String,
    pub block_id: Option<Uuid>,
    pub parent_comment_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
}

