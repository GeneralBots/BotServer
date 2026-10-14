use super::*;

pub(crate) async fn list_workspaces(
    State(state): State<Arc<WorkspacesState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Workspace>>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let branch_id = get_bot_context(&state);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = aiworkspaces::table
        .filter(aiworkspaces::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(search) = &query.search {
        let pattern = format!("%{search}%");
        q = q.filter(
            aiworkspaces::name
                .ilike(pattern.clone())
                .or(aiworkspaces::description.ilike(pattern)),
        );
    }

    let db_workspaces: Vec<DbWorkspace> = q
        .order(aiworkspaces::updated_at.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let mut result = Vec::with_capacity(db_workspaces.len());
    for ws in db_workspaces {
        let db_members: Vec<DbWorkspaceMember> = aiworkspace_members::table
            .filter(aiworkspace_members::workspace_id.eq(ws.id))
            .load(&mut conn)
            .unwrap_or_default();

        let members: Vec<WorkspaceMember> = db_members
            .into_iter()
            .map(|m| WorkspaceMember {
                user_id: m.user_id,
                role: WorkspaceRole::from_label(&m.role),
                joined_at: m.joined_at,
                invited_by: m.invited_by,
            })
            .collect();

        let root_pages: Vec<Uuid> = aiworkspace_pages::table
            .filter(aiworkspace_pages::workspace_id.eq(ws.id))
            .filter(aiworkspace_pages::parent_id.is_null())
            .select(aiworkspace_pages::id)
            .order(aiworkspace_pages::position.asc())
            .load(&mut conn)
            .unwrap_or_default();

        result.push(db_to_workspace(ws, members, root_pages));
    }

    Ok(Json(result))
}

pub(crate) async fn create_workspace(
    State(state): State<Arc<WorkspacesState>>,
    Json(req): Json<CreateWorkspaceRequest>,
) -> Result<Json<Workspace>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let branch_id = get_bot_context(&state);
    let id = Uuid::new_v4();
    let now = Utc::now();
    let user_id = Uuid::nil();

    let settings = WorkspaceSettings::default();
    let settings_json = serde_json::to_value(&settings).unwrap_or_else(|_| serde_json::json!({}));

    let db_workspace = DbWorkspace {
        id,
        branch_id,
        name: req.name,
        description: req.description,
        icon_type: None,
        icon_value: None,
        cover_image: None,
        settings: settings_json,
        created_by: user_id,
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(aiworkspaces::table)
        .values(&db_workspace)
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let member = DbWorkspaceMember {
        id: Uuid::new_v4(),
        workspace_id: id,
        user_id,
        role: WorkspaceRole::Owner.as_str().to_string(),
        invited_by: None,
        joined_at: now,
    };

    diesel::insert_into(aiworkspace_members::table)
        .values(&member)
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let members = vec![WorkspaceMember {
        user_id,
        role: WorkspaceRole::Owner,
        joined_at: now,
        invited_by: None,
    }];

    let workspace = db_to_workspace(db_workspace, members, vec![]);
    Ok(Json(workspace))
}

pub(crate) async fn get_workspace(
    State(state): State<Arc<WorkspacesState>>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Workspace>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let db_workspace: DbWorkspace = aiworkspaces::table
        .filter(aiworkspaces::id.eq(workspace_id))
        .first(&mut conn)
        .map_err(|_| WorkspacesError::WorkspaceNotFound)?;

    let db_members: Vec<DbWorkspaceMember> = aiworkspace_members::table
        .filter(aiworkspace_members::workspace_id.eq(workspace_id))
        .load(&mut conn)
        .unwrap_or_default();

    let members: Vec<WorkspaceMember> = db_members
        .into_iter()
        .map(|m| WorkspaceMember {
            user_id: m.user_id,
            role: WorkspaceRole::from_label(&m.role),
            joined_at: m.joined_at,
            invited_by: m.invited_by,
        })
        .collect();

    let root_pages: Vec<Uuid> = aiworkspace_pages::table
        .filter(aiworkspace_pages::workspace_id.eq(workspace_id))
        .filter(aiworkspace_pages::parent_id.is_null())
        .select(aiworkspace_pages::id)
        .order(aiworkspace_pages::position.asc())
        .load(&mut conn)
        .unwrap_or_default();

    let workspace = db_to_workspace(db_workspace, members, root_pages);
    Ok(Json(workspace))
}

pub(crate) async fn update_workspace(
    State(state): State<Arc<WorkspacesState>>,
    Path(workspace_id): Path<Uuid>,
    Json(req): Json<UpdateWorkspaceRequest>,
) -> Result<Json<Workspace>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let mut db_workspace: DbWorkspace = aiworkspaces::table
        .filter(aiworkspaces::id.eq(workspace_id))
        .first(&mut conn)
        .map_err(|_| WorkspacesError::WorkspaceNotFound)?;

    if let Some(name) = req.name {
        db_workspace.name = name;
    }
    if let Some(desc) = req.description {
        db_workspace.description = Some(desc);
    }
    if let Some(icon) = req.icon {
        db_workspace.icon_type = Some(icon.icon_type.as_str().to_string());
        db_workspace.icon_value = Some(icon.value);
    }
    db_workspace.updated_at = Utc::now();

    diesel::update(aiworkspaces::table.filter(aiworkspaces::id.eq(workspace_id)))
        .set(&db_workspace)
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let db_members: Vec<DbWorkspaceMember> = aiworkspace_members::table
        .filter(aiworkspace_members::workspace_id.eq(workspace_id))
        .load(&mut conn)
        .unwrap_or_default();

    let members: Vec<WorkspaceMember> = db_members
        .into_iter()
        .map(|m| WorkspaceMember {
            user_id: m.user_id,
            role: WorkspaceRole::from_label(&m.role),
            joined_at: m.joined_at,
            invited_by: m.invited_by,
        })
        .collect();

    let root_pages: Vec<Uuid> = aiworkspace_pages::table
        .filter(aiworkspace_pages::workspace_id.eq(workspace_id))
        .filter(aiworkspace_pages::parent_id.is_null())
        .select(aiworkspace_pages::id)
        .order(aiworkspace_pages::position.asc())
        .load(&mut conn)
        .unwrap_or_default();

    let workspace = db_to_workspace(db_workspace, members, root_pages);
    Ok(Json(workspace))
}

pub(crate) async fn delete_workspace(
    State(state): State<Arc<WorkspacesState>>,
    Path(workspace_id): Path<Uuid>,
) -> Result<StatusCode, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    diesel::delete(aiworkspace_comments::table.filter(aiworkspace_comments::workspace_id.eq(workspace_id)))
        .execute(&mut conn)
        .ok();

    let page_ids: Vec<Uuid> = aiworkspace_pages::table
        .filter(aiworkspace_pages::workspace_id.eq(workspace_id))
        .select(aiworkspace_pages::id)
        .load(&mut conn)
        .unwrap_or_default();

    if !page_ids.is_empty() {
        diesel::delete(aiworkspace_page_versions::table.filter(
            aiworkspace_page_versions::page_id.eq_any(&page_ids),
        ))
        .execute(&mut conn)
        .ok();
    }

    diesel::delete(aiworkspace_pages::table.filter(aiworkspace_pages::workspace_id.eq(workspace_id)))
        .execute(&mut conn)
        .ok();

    diesel::delete(aiworkspace_members::table.filter(aiworkspace_members::workspace_id.eq(workspace_id)))
        .execute(&mut conn)
        .ok();

    let deleted = diesel::delete(aiworkspaces::table.filter(aiworkspaces::id.eq(workspace_id)))
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    if deleted > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(WorkspacesError::WorkspaceNotFound)
    }
}

pub(crate) async fn list_pages(
    State(state): State<Arc<WorkspacesState>>,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<PageTreeNode>>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let db_pages: Vec<DbWorkspacePage> = aiworkspace_pages::table
        .filter(aiworkspace_pages::workspace_id.eq(workspace_id))
        .order(aiworkspace_pages::position.asc())
        .load(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    fn build_tree(pages: &[DbWorkspacePage], parent_id: Option<Uuid>) -> Vec<PageTreeNode> {
        pages
            .iter()
            .filter(|p| p.parent_id == parent_id)
            .map(|p| {
                let icon = match (&p.icon_type, &p.icon_value) {
                    (Some(t), Some(v)) => Some(WorkspaceIcon {
                        icon_type: IconType::from_label(t),
                        value: v.clone(),
                    }),
                    _ => None,
                };
                let children = build_tree(pages, Some(p.id));
                PageTreeNode {
                    id: p.id,
                    title: p.title.clone(),
                    icon,
                    has_children: !children.is_empty(),
                    children,
                }
            })
            .collect()
    }

    let tree = build_tree(&db_pages, None);
    Ok(Json(tree))
}

