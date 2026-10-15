use super::*;

pub async fn handle_trending_topics(
    State(state): State<Arc<SocialState>>,
) -> Result<Json<Vec<TrendingTopic>>, SocialError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let topics = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);

        let db_posts: Vec<DbPost> = social_posts::table
            .filter(social_posts::bot_id.eq(bot_id))
            .filter(social_posts::deleted_at.is_null())
            .order(social_posts::created_at.desc())
            .limit(300)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        let mut counts: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        for p in &db_posts {
            for tag in p.hashtags.iter().flatten() {
                if !tag.is_empty() {
                    *counts.entry(tag.clone()).or_insert(0) += 1;
                }
            }
        }

        let mut ranked: Vec<TrendingTopic> = counts
            .into_iter()
            .map(|(tag, count)| TrendingTopic { tag, count })
            .collect();
        ranked.sort_by_key(|a| std::cmp::Reverse(a.count));
        ranked.truncate(10);
        Ok::<_, SocialError>(ranked)
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(topics))
}
pub async fn handle_recent_activity(
    State(state): State<Arc<SocialState>>,
) -> Result<Json<Vec<ActivityItem>>, SocialError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let items = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);

        let db_posts: Vec<DbPost> = social_posts::table
            .filter(social_posts::bot_id.eq(bot_id))
            .filter(social_posts::deleted_at.is_null())
            .order(social_posts::created_at.desc())
            .limit(15)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        let activity: Vec<ActivityItem> = db_posts
            .into_iter()
            .map(|p| {
                let summary = if p.content.chars().count() > 80 {
                    format!("{}…", p.content.chars().take(80).collect::<String>())
                } else {
                    p.content.clone()
                };
                ActivityItem {
                    id: p.id,
                    author_id: p.author_id,
                    kind: if p.is_announcement {
                        "announcement".to_string()
                    } else {
                        "post".to_string()
                    },
                    summary,
                    created_at: p.created_at,
                }
            })
            .collect();

        Ok::<_, SocialError>(activity)
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(items))
}
pub async fn handle_people_suggestions(
    State(state): State<Arc<SocialState>>,
) -> Result<Json<Vec<PersonSuggestion>>, SocialError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let people = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);

        let authors: Vec<Uuid> = social_posts::table
            .filter(social_posts::bot_id.eq(bot_id))
            .filter(social_posts::deleted_at.is_null())
            .select(social_posts::author_id)
            .distinct()
            .limit(10)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        let suggestions: Vec<PersonSuggestion> = authors
            .into_iter()
            .map(|id| PersonSuggestion {
                user_id: id,
                name: "Member".to_string(),
                avatar_url: None,
            })
            .collect();

        Ok::<_, SocialError>(suggestions)
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(people))
}
pub async fn handle_my_communities(
    State(state): State<Arc<SocialState>>,
) -> Result<Json<Vec<MyCommunity>>, SocialError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let communities = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);

        let member_community_ids: Vec<Uuid> = social_community_members::table
            .filter(social_community_members::user_id.eq(Uuid::nil()))
            .select(social_community_members::community_id)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        let db_communities: Vec<DbCommunity> = social_communities::table
            .filter(social_communities::bot_id.eq(bot_id))
            .filter(social_communities::id.eq_any(member_community_ids))
            .filter(social_communities::archived_at.is_null())
            .order(social_communities::member_count.desc())
            .limit(20)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        let mapped: Vec<MyCommunity> = db_communities
            .into_iter()
            .map(|c| MyCommunity {
                id: c.id,
                name: c.name,
                slug: c.slug,
                icon: c.icon,
                member_count: c.member_count,
            })
            .collect();

        Ok::<_, SocialError>(mapped)
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(communities))
}
pub fn configure_social_routes() -> Router<Arc<SocialState>> {
    Router::new()
        .route("/api/social/feed", get(handle_get_feed))
        .route("/api/social/announcements", get(handle_list_announcements))
        .route("/api/social/trending", get(handle_trending_topics))
        .route("/api/social/activity", get(handle_recent_activity))
        .route("/api/social/people", get(handle_people_suggestions))
        .route("/api/social/my-communities", get(handle_my_communities))
        .route("/api/ui/social/suggested", get(handle_get_suggested_communities_html))
        .route("/api/social/posts", post(handle_create_post))
        .route("/api/social/posts/:id", get(handle_get_post).put(handle_update_post).delete(handle_delete_post))
        .route("/api/social/posts/:id/react", post(handle_add_reaction))
        .route("/api/social/posts/:id/react/:type", delete(handle_remove_reaction))
        .route("/api/social/posts/:id/comments", get(handle_get_comments).post(handle_add_comment))
        .route("/api/social/communities", get(handle_list_communities).post(handle_create_community))
        .route("/api/social/communities/:id", get(handle_get_community))
        .route("/api/social/communities/:id/join", post(handle_join_community))
        .route("/api/social/communities/:id/leave", post(handle_leave_community))
        .route("/api/social/praise", post(handle_send_praise))
        .route("/api/social/analytics", get(handle_social_analytics))
}
pub fn configure_social_ui_routes() -> Router<Arc<SocialState>> {
    Router::new()
        .route("/suite/social", get(handle_social_list_page))
        .route("/suite/social/compose", get(handle_social_compose_page))
        .route("/suite/social/:id", get(handle_social_post_page))
}
