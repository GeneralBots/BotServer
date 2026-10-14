use super::*;

pub(crate) async fn create_page(
    State(state): State<Arc<WorkspacesState>>,
    Path(workspace_id): Path<Uuid>,
    Json(req): Json<CreatePageRequest>,
) -> Result<Json<Page>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let _: DbWorkspace = aiworkspaces::table
        .filter(aiworkspaces::id.eq(workspace_id))
        .first(&mut conn)
        .map_err(|_| WorkspacesError::WorkspaceNotFound)?;

    let now = Utc::now();
    let user_id = Uuid::nil();
    let id = Uuid::new_v4();

    let max_position: Option<i32> = aiworkspace_pages::table
        .filter(aiworkspace_pages::workspace_id.eq(workspace_id))
        .filter(aiworkspace_pages::parent_id.is_not_distinct_from(req.parent_id))
        .select(diesel::dsl::max(aiworkspace_pages::position))
        .first(&mut conn)
        .ok()
        .flatten();

    let db_page = DbWorkspacePage {
        id,
        workspace_id,
        parent_id: req.parent_id,
        title: req.title,
        icon_type: None,
        icon_value: None,
        cover_image: None,
        content: serde_json::json!([]),
        properties: serde_json::json!({}),
        is_template: false,
        template_id: None,
        is_public: false,
        public_edit: false,
        position: max_position.unwrap_or(0) + 1,
        created_by: user_id,
        last_edited_by: Some(user_id),
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(aiworkspace_pages::table)
        .values(&db_page)
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let page = db_to_page(db_page, vec![]);
    Ok(Json(page))
}

pub(crate) async fn get_page(
    State(state): State<Arc<WorkspacesState>>,
    Path(page_id): Path<Uuid>,
) -> Result<Json<Page>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let db_page: DbWorkspacePage = aiworkspace_pages::table
        .filter(aiworkspace_pages::id.eq(page_id))
        .first(&mut conn)
        .map_err(|_| WorkspacesError::PageNotFound)?;

    let children: Vec<Uuid> = aiworkspace_pages::table
        .filter(aiworkspace_pages::parent_id.eq(page_id))
        .select(aiworkspace_pages::id)
        .order(aiworkspace_pages::position.asc())
        .load(&mut conn)
        .unwrap_or_default();

    let page = db_to_page(db_page, children);
    Ok(Json(page))
}

pub(crate) async fn update_page(
    State(state): State<Arc<WorkspacesState>>,
    Path(page_id): Path<Uuid>,
    Json(req): Json<UpdatePageRequest>,
) -> Result<Json<Page>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let mut db_page: DbWorkspacePage = aiworkspace_pages::table
        .filter(aiworkspace_pages::id.eq(page_id))
        .first(&mut conn)
        .map_err(|_| WorkspacesError::PageNotFound)?;

    if let Some(title) = req.title {
        db_page.title = title;
    }
    if let Some(icon) = req.icon {
        db_page.icon_type = Some(icon.icon_type.as_str().to_string());
        db_page.icon_value = Some(icon.value);
    }
    if let Some(blocks) = req.blocks {
        db_page.content = serde_json::to_value(&blocks).unwrap_or_else(|_| serde_json::json!([]));
    }
    db_page.updated_at = Utc::now();
    db_page.last_edited_by = Some(Uuid::nil());

    diesel::update(aiworkspace_pages::table.filter(aiworkspace_pages::id.eq(page_id)))
        .set(&db_page)
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let children: Vec<Uuid> = aiworkspace_pages::table
        .filter(aiworkspace_pages::parent_id.eq(page_id))
        .select(aiworkspace_pages::id)
        .order(aiworkspace_pages::position.asc())
        .load(&mut conn)
        .unwrap_or_default();

    let page = db_to_page(db_page, children);
    Ok(Json(page))
}

pub(crate) async fn delete_page(
    State(state): State<Arc<WorkspacesState>>,
    Path(page_id): Path<Uuid>,
) -> Result<StatusCode, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    diesel::delete(aiworkspace_comments::table.filter(aiworkspace_comments::page_id.eq(page_id)))
        .execute(&mut conn)
        .ok();

    diesel::delete(aiworkspace_page_versions::table.filter(aiworkspace_page_versions::page_id.eq(page_id)))
        .execute(&mut conn)
        .ok();

    diesel::delete(aiworkspace_pages::table.filter(aiworkspace_pages::parent_id.eq(page_id)))
        .execute(&mut conn)
        .ok();

    let deleted = diesel::delete(aiworkspace_pages::table.filter(aiworkspace_pages::id.eq(page_id)))
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    if deleted > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(WorkspacesError::PageNotFound)
    }
}

pub(crate) async fn add_member(
    State(state): State<Arc<WorkspacesState>>,
    Path(workspace_id): Path<Uuid>,
    Json(req): Json<AddMemberRequest>,
) -> Result<StatusCode, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let existing: Option<DbWorkspaceMember> = aiworkspace_members::table
        .filter(aiworkspace_members::workspace_id.eq(workspace_id))
        .filter(aiworkspace_members::user_id.eq(req.user_id))
        .first(&mut conn)
        .optional()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    if existing.is_some() {
        return Err(WorkspacesError::MemberAlreadyExists);
    }

    let now = Utc::now();
    let member = DbWorkspaceMember {
        id: Uuid::new_v4(),
        workspace_id,
        user_id: req.user_id,
        role: req.role.as_str().to_string(),
        invited_by: Some(Uuid::nil()),
        joined_at: now,
    };

    diesel::insert_into(aiworkspace_members::table)
        .values(&member)
        .execute(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    Ok(StatusCode::CREATED)
}

pub(crate) async fn remove_member(
    State(state): State<Arc<WorkspacesState>>,
    Path((workspace_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let owner_count: i64 = aiworkspace_members::table
        .filter(aiworkspace_members::workspace_id.eq(workspace_id))
        .filter(aiworkspace_members::role.eq("owner"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let member: Option<DbWorkspaceMember> = aiworkspace_members::table
        .filter(aiworkspace_members::workspace_id.eq(workspace_id))
        .filter(aiworkspace_members::user_id.eq(user_id))
        .first(&mut conn)
        .optional()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    if let Some(m) = member {
        if m.role == "owner" && owner_count <= 1 {
            return Err(WorkspacesError::CannotRemoveLastOwner);
        }
    } else {
        return Err(WorkspacesError::MemberNotFound);
    }

    diesel::delete(
        aiworkspace_members::table
            .filter(aiworkspace_members::workspace_id.eq(workspace_id))
            .filter(aiworkspace_members::user_id.eq(user_id)),
    )
    .execute(&mut conn)
    .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn search_pages(
    State(state): State<Arc<WorkspacesState>>,
    Path(workspace_id): Path<Uuid>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<PageSearchResult>>, WorkspacesError> {
    let mut conn = state
        .pool
        .get()
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let query = params.q.unwrap_or_default();
    if query.is_empty() {
        return Ok(Json(vec![]));
    }

    let pattern = format!("%{query}%");
    let db_pages: Vec<DbWorkspacePage> = aiworkspace_pages::table
        .filter(aiworkspace_pages::workspace_id.eq(workspace_id))
        .filter(aiworkspace_pages::title.ilike(&pattern))
        .order(aiworkspace_pages::updated_at.desc())
        .limit(20)
        .load(&mut conn)
        .map_err(|e| WorkspacesError::DbError(e.to_string()))?;

    let results: Vec<PageSearchResult> = db_pages
        .into_iter()
        .map(|p| {
            let icon = match (&p.icon_type, &p.icon_value) {
                (Some(t), Some(v)) => Some(WorkspaceIcon {
                    icon_type: IconType::from_label(t),
                    value: v.clone(),
                }),
                _ => None,
            };
            PageSearchResult {
                page_id: p.id,
                title: p.title,
                icon,
                snippet: String::new(),
                updated_at: p.updated_at,
            }
        })
        .collect();

    Ok(Json(results))
}

pub(crate) async fn get_slash_commands_handler(
    State(_state): State<Arc<WorkspacesState>>,
) -> Json<Vec<SlashCommand>> {
    Json(vec![
        SlashCommand {
            id: "paragraph".to_string(),
            name: "Text".to_string(),
            description: "Plain text paragraph".to_string(),
            icon: "type".to_string(),
            category: SlashCommandCategory::General,
            keywords: vec!["text".to_string(), "paragraph".to_string()],
        },
        SlashCommand {
            id: "heading1".to_string(),
            name: "Heading 1".to_string(),
            description: "Large section heading".to_string(),
            icon: "heading-1".to_string(),
            category: SlashCommandCategory::General,
            keywords: vec!["h1".to_string(), "heading".to_string()],
        },
        SlashCommand {
            id: "heading2".to_string(),
            name: "Heading 2".to_string(),
            description: "Medium section heading".to_string(),
            icon: "heading-2".to_string(),
            category: SlashCommandCategory::General,
            keywords: vec!["h2".to_string(), "heading".to_string()],
        },
        SlashCommand {
            id: "bulleted_list".to_string(),
            name: "Bulleted list".to_string(),
            description: "Create a bulleted list".to_string(),
            icon: "list".to_string(),
            category: SlashCommandCategory::General,
            keywords: vec!["bullet".to_string(), "list".to_string()],
        },
        SlashCommand {
            id: "numbered_list".to_string(),
            name: "Numbered list".to_string(),
            description: "Create a numbered list".to_string(),
            icon: "list-ordered".to_string(),
            category: SlashCommandCategory::General,
            keywords: vec!["number".to_string(), "list".to_string()],
        },
        SlashCommand {
            id: "checklist".to_string(),
            name: "Checklist".to_string(),
            description: "Create a checklist".to_string(),
            icon: "check-square".to_string(),
            category: SlashCommandCategory::General,
            keywords: vec!["todo".to_string(), "checkbox".to_string()],
        },
        SlashCommand {
            id: "code".to_string(),
            name: "Code".to_string(),
            description: "Create a code block".to_string(),
            icon: "code".to_string(),
            category: SlashCommandCategory::General,
            keywords: vec!["code".to_string(), "snippet".to_string()],
        },
        SlashCommand {
            id: "image".to_string(),
            name: "Image".to_string(),
            description: "Upload or embed an image".to_string(),
            icon: "image".to_string(),
            category: SlashCommandCategory::Media,
            keywords: vec!["image".to_string(), "picture".to_string()],
        },
    ])
}

