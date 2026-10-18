use super::*;

pub async fn handle_add_comment(
    State(state): State<Arc<SocialState>>,
    Path(post_id): Path<Uuid>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<Comment>, SocialError> {
    let pool = state.pool.clone();
    let user_id = Uuid::nil();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let now = Utc::now();

        let db_comment = DbComment {
            id: Uuid::new_v4(),
            post_id,
            parent_comment_id: req.parent_comment_id,
            author_id: user_id,
            content: req.content,
            mentions: serde_json::to_value(req.mentions.unwrap_or_default()).unwrap_or_default(),
            reaction_counts: serde_json::json!({}),
            reply_count: 0,
            created_at: now,
            updated_at: now,
            edited_at: None,
            deleted_at: None,
        };

        diesel::insert_into(social_comments::table)
            .values(&db_comment)
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        diesel::update(social_posts::table.filter(social_posts::id.eq(post_id)))
            .set(social_posts::comment_count.eq(social_posts::comment_count + 1))
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(db_comment_to_comment(db_comment))
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub async fn handle_send_praise(
    State(state): State<Arc<SocialState>>,
    Json(req): Json<CreatePraiseRequest>,
) -> Result<Json<Praise>, SocialError> {
    let pool = state.pool.clone();
    let from_user_id = Uuid::nil();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);
        let org_id = Uuid::nil();
        let branch_id = crate::branch_for_bot(&mut conn, bot_id);
        let now = Utc::now();

        let db_praise = DbPraise {
            id: Uuid::new_v4(),
            branch_id,
            org_id,
            bot_id,
            from_user_id,
            to_user_id: req.to_user_id,
            badge_type: req.badge_type.clone(),
            message: Some(req.message.clone()),
            is_public: req.is_public.unwrap_or(true),
            post_id: None,
            created_at: now,
        };

        diesel::insert_into(social_praises::table)
            .values(&db_praise)
            .execute(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(Praise {
            id: db_praise.id,
            organization_id: org_id,
            from_user_id,
            to_user_id: req.to_user_id,
            badge_type: req.badge_type,
            message: req.message,
            is_public: db_praise.is_public,
            post_id: None,
            created_at: now,
        })
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
pub(super) fn render_post_card_html(post: &PostWithAuthor) -> String {
    let reactions_html: String = post
        .post
        .reaction_counts
        .iter()
        .map(|(emoji, count)| format!(r#"<span class="reaction">{emoji} {count}</span>"#))
        .collect();

    let avatar_url = post.author.avatar_url.as_deref().unwrap_or("/assets/default-avatar.svg");
    let post_time = post.post.created_at.format("%b %d, %Y");
    let post_id_str = post.post.id.to_string();
    let comments_id = format!("comments-{post_id_str}");
    let comments_target = format!("#comments-{post_id_str}");

    format!(
        r#"<article class="post-card" data-post-id="{id}">
<header class="post-header">
<img class="avatar" src="{avatar}" alt="{name}" />
<div class="post-meta"><span class="author-name">{name}</span><span class="post-time">{time}</span></div>
</header>
<div class="post-content">{content}</div>
<footer class="post-footer">
<div class="reactions">{reactions}</div>
<div class="post-actions">
<button class="btn-react" hx-post="/api/social/posts/{id}/react" hx-swap="outerHTML">Like</button>
<button class="btn-comment" hx-get="/api/social/posts/{id}/comments" hx-target="{comments_target}">Comment {comment_count}</button>
</div>
</footer>
<div id="{comments_id}" class="comments-section"></div>
</article>"#,
        id = post.post.id,
        avatar = avatar_url,
        name = post.author.name,
        time = post_time,
        content = post.post.content,
        reactions = reactions_html,
        comment_count = post.post.comment_count,
    )
}
pub(super) async fn handle_get_suggested_communities_html(
    State(state): State<Arc<SocialState>>,
) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let communities = tokio::task::spawn_blocking(move || {
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let (bot_id, _) = get_default_bot(&mut conn);

        social_communities::table
            .filter(social_communities::bot_id.eq(bot_id))
            .filter(social_communities::archived_at.is_null())
            .filter(social_communities::visibility.eq("public"))
            .order(social_communities::member_count.desc())
            .limit(5)
            .load::<DbCommunity>(&mut conn)
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    if communities.is_empty() {
        return Html(r#"<div class="empty-suggestions"><p>No communities available</p></div>"#.to_string());
    }

    let html: String = communities
        .into_iter()
        .map(|c| {
            format!(
                r#"<div class="community-suggestion">
<div class="community-avatar">{}</div>
<div class="community-info">
<span class="community-name">{}</span>
<span class="community-members">{} members</span>
</div>
<button class="btn-join" hx-post="/api/social/communities/{}/join" hx-swap="outerHTML">Join</button>
</div>"#,
                c.icon.as_deref().unwrap_or("🌐"),
                c.name,
                c.member_count,
                c.id
            )
        })
        .collect();

    Html(html)
}
pub async fn handle_social_list_page(State(_state): State<Arc<SocialState>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Social Media Manager</title><style>
*{box-sizing:border-box;margin:0;padding:0}body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#f5f5f5}.container{max-width:1400px;margin:0 auto;padding:24px}.header{display:flex;justify-content:space-between;align-items:center;margin-bottom:24px}.header h1{font-size:28px;color:#1a1a1a}.btn{padding:10px 20px;border:none;border-radius:8px;cursor:pointer;font-size:14px;font-weight:500}.btn-primary{background:#0066cc;color:white}.stats-row{display:grid;grid-template-columns:repeat(4,1fr);gap:16px;margin-bottom:24px}.stat-card{background:white;border-radius:12px;padding:20px;box-shadow:0 2px 8px rgba(0,0,0,.08)}.stat-value{font-size:28px;font-weight:600;color:#1a1a1a}.stat-label{font-size:13px;color:#666;margin-top:4px}.empty-state{text-align:center;padding:40px;color:#666}
</style></head><body><div class="container"><div class="header"><h1>Social Media Manager</h1><button class="btn btn-primary" onclick="createPost()">Create Post</button></div><div class="stats-row"><div class="stat-card"><div class="stat-value" id="totalFollowers">0</div><div class="stat-label">Total Followers</div></div><div class="stat-card"><div class="stat-value" id="totalEngagement">0</div><div class="stat-label">Engagement Rate</div></div><div class="stat-card"><div class="stat-value" id="postsThisWeek">0</div><div class="stat-label">Posts This Week</div></div><div class="stat-card"><div class="stat-value" id="totalReach">0</div><div class="stat-label">Total Reach</div></div></div><div id="postsList"><div class="empty-state">No posts yet.</div></div></div><script>function createPost(){window.location='/suite/social/compose';}</script></body></html>"#;
    Html(html.to_string())
}
pub async fn handle_social_compose_page(State(_state): State<Arc<SocialState>>) -> Html<String> {
    let html = r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Compose Post</title><style>
*{box-sizing:border-box;margin:0;padding:0}body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#f5f5f5}.container{max-width:900px;margin:0 auto;padding:24px}.compose-card{background:white;border-radius:12px;padding:32px;box-shadow:0 2px 8px rgba(0,0,0,.08)}h1{font-size:24px;margin-bottom:24px}.form-group{margin-bottom:20px}.form-group label{display:block;font-weight:500;margin-bottom:8px}.form-group textarea{width:100%;padding:16px;border:1px solid #ddd;border-radius:8px;font-size:16px;min-height:150px;resize:vertical}.btn{padding:12px 24px;border:none;border-radius:8px;cursor:pointer;font-size:14px;font-weight:500}.btn-primary{background:#0066cc;color:white}.form-actions{display:flex;gap:12px;justify-content:flex-end}
</style></head><body><div class="container"><a href="/suite/social" style="color:#0066cc;text-decoration:none;">&#8592; Back to Social</a><div class="compose-card"><h1>Compose Post</h1><form id="composeForm"><div class="form-group"><label>Post Content</label><textarea id="content" placeholder="What's on your mind?"></textarea></div><div class="form-actions"><button type="submit" class="btn btn-primary">Post Now</button></div></form></div></div><script>
document.getElementById('composeForm').addEventListener('submit',async(e)=>{e.preventDefault();const data={content:document.getElementById('content').value};try{const r=await fetch('/api/social/posts',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(data)});if(r.ok)window.location='/suite/social';else alert('Failed');}catch(e){alert('Error: '+e.message);}});
</script></body></html>"#;
    Html(html.to_string())
}
pub async fn handle_social_post_page(
    State(_state): State<Arc<SocialState>>,
    Path(post_id): Path<Uuid>,
) -> Html<String> {
    let post_id_safe = post_id;
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Post Details</title><style>
*{{box-sizing:border-box;margin:0;padding:0}}body{{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:#f5f5f5}}.container{{max-width:900px;margin:0 auto;padding:24px}}.post-card{{background:white;border-radius:12px;padding:32px;box-shadow:0 2px 8px rgba(0,0,0,.08);margin-bottom:24px}}.post-content{{font-size:18px;line-height:1.6;margin-bottom:20px}}.btn{{padding:10px 20px;border:none;border-radius:8px;cursor:pointer;font-size:14px}}.btn-danger{{background:#ffebee;color:#c62828}}
</style></head><body><div class="container"><a href="/suite/social" style="color:#0066cc;text-decoration:none;">&#8592; Back to Social</a><div class="post-card"><div class="post-content" id="postContent">Loading...</div><button class="btn btn-danger" onclick="deletePost()">Delete Post</button></div></div><script>
const postId='{post_id_safe}';async function loadPost(){{try{{const r=await fetch(`/api/social/posts/${{postId}}`);const p=await r.json();if(p)document.getElementById('postContent').textContent=p.content;}}catch(e){{console.error(e);}}}}async function deletePost(){{if(!confirm('Delete this post?'))return;try{{await fetch(`/api/social/posts/${{postId}}`,{{method:'DELETE'}});window.location='/suite/social';}}catch(e){{alert('Failed');}}}}loadPost();
</script></body></html>"#
    );
    Html(html)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendingTopic {
    pub tag: String,
    pub count: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonSuggestion {
    pub user_id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityItem {
    pub id: Uuid,
    pub author_id: Uuid,
    pub kind: String,
    pub summary: String,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyCommunity {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub icon: Option<String>,
    pub member_count: i32,
}
pub async fn handle_list_announcements(
    State(state): State<Arc<SocialState>>,
) -> Result<Json<Vec<Post>>, SocialError> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| SocialError::Database(e.to_string()))?;
        let (bot_id, _) = get_default_bot(&mut conn);

        let db_posts: Vec<DbPost> = social_posts::table
            .filter(social_posts::bot_id.eq(bot_id))
            .filter(social_posts::is_announcement)
            .filter(social_posts::deleted_at.is_null())
            .order(social_posts::created_at.desc())
            .limit(50)
            .load(&mut conn)
            .map_err(|e| SocialError::Database(e.to_string()))?;

        Ok::<_, SocialError>(db_posts.into_iter().map(db_post_to_post).collect())
    })
    .await
    .map_err(|e| SocialError::Internal(e.to_string()))??;

    Ok(Json(result))
}
