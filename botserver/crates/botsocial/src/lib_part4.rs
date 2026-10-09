use super::*;

pub async fn handle_delete_post(
    State(state): State<Arc<SocialState>>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, SocialError> {
    let pool = state.pool.clone();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let now = Utc::now();

        diesel::update(social_posts::table.filter(social_posts::id.eq(post_id)))
            .set(social_posts::deleted_at.eq(Some(now)))
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(())
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(serde_json::json!({ "success": true })))
}
pub async fn handle_list_communities(
    State(state): State<Arc<SocialState>>,
) -> Result<Json<Vec<Community>>, SocialError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);

        let db_communities: Vec<DbCommunity> = social_communities::table
            .filter(social_communities::bot_id.eq(bot_id))
            .filter(social_communities::archived_at.is_null())
            .order(social_communities::member_count.desc())
            .limit(50)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(db_communities.into_iter().map(db_community_to_community).collect())
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_create_community(
    State(state): State<Arc<SocialState>>,
    Json(req): Json<CreateCommunityRequest>,
) -> Result<Json<Community>, SocialError> {
    let pool = state.pool.clone();
    let user_id = Uuid::nil();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);
        let org_id = Uuid::nil();
        let now = Utc::now();

        let slug = req
            .slug
            .unwrap_or_else(|| req.name.to_lowercase().replace(' ', "-"));

        let db_community = DbCommunity {
            id: Uuid::new_v4(),
            org_id,
            bot_id,
            name: req.name,
            slug,
            description: req.description,
            cover_image: None,
            icon: None,
            visibility: req.visibility.unwrap_or_else(|| "private".to_string()),
            join_policy: req.join_policy.unwrap_or_else(|| "open".to_string()),
            owner_id: user_id,
            member_count: 1,
            post_count: 0,
            is_official: false,
            is_featured: false,
            settings: serde_json::json!({}),
            created_at: now,
            updated_at: now,
            archived_at: None,
        };

        diesel::insert_into(social_communities::table)
            .values(&db_community)
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        let member = DbCommunityMember {
            id: Uuid::new_v4(),
            community_id: db_community.id,
            user_id,
            role: "owner".to_string(),
            notifications_enabled: true,
            joined_at: now,
            last_seen_at: Some(now),
        };

        diesel::insert_into(social_community_members::table)
            .values(&member)
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(db_community_to_community(db_community))
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_get_community(
    State(state): State<Arc<SocialState>>,
    Path(community_id): Path<Uuid>,
) -> Result<Json<Community>, SocialError> {
    let pool = state.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;

        let db_community: DbCommunity = social_communities::table
            .filter(social_communities::id.eq(community_id))
            .filter(social_communities::archived_at.is_null())
            .first(&mut conn)
            .map_err(|_| SocialError::NotFound("Community not found".to_string()))?;

        Ok::<_, SocialError>(db_community_to_community(db_community))
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_join_community(
    State(state): State<Arc<SocialState>>,
    Path(community_id): Path<Uuid>,
) -> Result<Json<CommunityMember>, SocialError> {
    let pool = state.pool.clone();
    let user_id = Uuid::nil();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let now = Utc::now();

        let member = DbCommunityMember {
            id: Uuid::new_v4(),
            community_id,
            user_id,
            role: "member".to_string(),
            notifications_enabled: true,
            joined_at: now,
            last_seen_at: Some(now),
        };

        diesel::insert_into(social_community_members::table)
            .values(&member)
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        diesel::update(social_communities::table.filter(social_communities::id.eq(community_id)))
            .set(social_communities::member_count.eq(social_communities::member_count + 1))
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(db_member_to_member(member))
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_leave_community(
    State(state): State<Arc<SocialState>>,
    Path(community_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, SocialError> {
    let pool = state.pool.clone();
    let user_id = Uuid::nil();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;

        diesel::delete(
            social_community_members::table
                .filter(social_community_members::community_id.eq(community_id))
                .filter(social_community_members::user_id.eq(user_id)),
        )
        .execute(&mut conn)
        .map_err(|e| SocialError::Database(e.to_string()))?;

        diesel::update(social_communities::table.filter(social_communities::id.eq(community_id)))
            .set(social_communities::member_count.eq(social_communities::member_count - 1))
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(())
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(serde_json::json!({ "success": true })))
}
pub async fn handle_add_reaction(
    State(state): State<Arc<SocialState>>,
    Path(post_id): Path<Uuid>,
    Json(req): Json<CreateReactionRequest>,
) -> Result<Json<Reaction>, SocialError> {
    let pool = state.pool.clone();
    let user_id = Uuid::nil();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let now = Utc::now();

        let db_reaction = DbReaction {
            id: Uuid::new_v4(),
            post_id: Some(post_id),
            comment_id: None,
            user_id,
            reaction_type: req.reaction_type.clone(),
            created_at: now,
        };

        diesel::insert_into(social_reactions::table)
            .values(&db_reaction)
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(Reaction {
            id: db_reaction.id,
            post_id,
            user_id,
            reaction_type: req.reaction_type,
            created_at: now,
        })
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_remove_reaction(
    State(state): State<Arc<SocialState>>,
    Path((post_id, reaction_type)): Path<(Uuid, String)>,
) -> Result<Json<serde_json::Value>, SocialError> {
    let pool = state.pool.clone();
    let user_id = Uuid::nil();

    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;

        diesel::delete(
            social_reactions::table
                .filter(social_reactions::post_id.eq(post_id))
                .filter(social_reactions::user_id.eq(user_id))
                .filter(social_reactions::reaction_type.eq(reaction_type)),
        )
        .execute(&mut conn)
        .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(())
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(serde_json::json!({ "success": true })))
}
pub async fn handle_get_comments(
    State(state): State<Arc<SocialState>>,
    Path(post_id): Path<Uuid>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<Vec<Comment>>, SocialError> {
    let pool = state.pool.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;

        let limit = query.limit.unwrap_or(20);
        let offset = query.offset.unwrap_or(0);

        let db_comments: Vec<DbComment> = social_comments::table
            .filter(social_comments::post_id.eq(post_id))
            .filter(social_comments::deleted_at.is_null())
            .order(social_comments::created_at.asc())
            .offset(offset)
            .limit(limit)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(db_comments.into_iter().map(db_comment_to_comment).collect())
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
