use super::*;

pub(super) fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
pub fn configure_people_ui_routes() -> Router<Arc<PeopleState>> {
    Router::new()
        .route("/api/ui/people", get(handle_people_list))
        .route("/api/ui/people/count", get(handle_people_count))
        .route("/api/ui/people/active-count", get(handle_active_count))
        .route("/api/ui/people/cards", get(handle_people_cards))
        .route("/api/ui/people/search", get(handle_people_search))
        .route("/api/ui/people/:id", get(handle_person_detail))
        .route("/api/ui/people/departments", get(handle_departments_list))
        .route("/api/ui/people/teams", get(handle_teams_list))
        .route("/api/ui/people/time-off", get(handle_time_off_list))
        .route("/api/ui/people/stats", get(handle_people_stats))
        .route("/api/ui/people/new", get(handle_new_person_form))
}
pub(super) async fn handle_people_list(
    State(state): State<Arc<PeopleState>>,
    Query(query): Query<PeopleQuery>,
) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result: Option<Vec<PersonListRow>> = tokio::task::spawn_blocking(move || -> Option<Vec<PersonListRow>> {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        let mut db_query = people::table
            .filter(people::branch_id.eq(branch_id))
            .into_boxed();

        if let Some(ref dept) = query.department {
            db_query = db_query.filter(people::department.eq(dept));
        }

        if let Some(is_active) = query.is_active {
            db_query = db_query.filter(people::is_active.eq(is_active));
        }

        if let Some(ref search) = query.search {
            let term = format!("%{search}%");
            let term2 = term.clone();
            let term3 = term.clone();
            db_query = db_query.filter(
                people::first_name.ilike(term)
                    .or(people::last_name.ilike(term2))
                    .or(people::email.ilike(term3)),
            );
        }

        db_query = db_query.order(people::first_name.asc());

        let limit = query.limit.unwrap_or(50);
        db_query = db_query.limit(limit);

        db_query
            .select((
                people::id,
                people::first_name,
                people::last_name,
                people::email,
                people::job_title,
                people::department,
                people::avatar_url,
                people::is_active,
            ))
            .load::<PersonListRow>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(persons) if !persons.is_empty() => {
            let mut html = String::from(
                r##"<table class="people-table">
<thead>
<tr>
<th>Name</th>
<th>Email</th>
<th>Job Title</th>
<th>Department</th>
<th>Status</th>
<th>Actions</th>
</tr>
</thead>
<tbody>"##,
            );

            for row in persons {
                let full_name = format!("{} {}", row.first_name, row.last_name.unwrap_or_default());
                let email_str = row.email.unwrap_or_else(|| "-".to_string());
                let title_str = row.job_title.unwrap_or_else(|| "-".to_string());
                let dept_str = row.department.unwrap_or_else(|| "-".to_string());
                let avatar = row.avatar_url.unwrap_or_else(|| "/assets/default-avatar.png".to_string());
                let is_active = row.is_active;
                let id = row.id;
                let status_class = if is_active { "status-active" } else { "status-inactive" };
                let status_text = if is_active { "Active" } else { "Inactive" };

                html.push_str(&format!(
                    r##"<tr class="person-row" data-id="{id}">
<td class="person-name">
<img src="{}" class="avatar-sm" alt="" />
<span>{}</span>
</td>
<td class="person-email">{}</td>
<td class="person-title">{}</td>
<td class="person-department">{}</td>
<td class="person-status"><span class="{}">{}</span></td>
<td class="person-actions">
<button class="btn-sm" hx-get="/api/ui/people/{id}" hx-target="#person-detail">View</button>
<button class="btn-sm btn-secondary" hx-get="/api/people/{id}/edit" hx-target="#modal-content">Edit</button>
</td>
</tr>"##,
                    html_escape(&avatar),
                    html_escape(&full_name),
                    html_escape(&email_str),
                    html_escape(&title_str),
                    html_escape(&dept_str),
                    status_class,
                    status_text,
                ));
            }

            html.push_str("</tbody></table>");
            Html(html)
        }
        _ => Html(
            r##"<div class="empty-state">
<div class="empty-icon">👥</div>
<p>No people found</p>
<p class="empty-hint">Add people to your directory</p>
<button class="btn btn-primary" hx-get="/api/ui/people/new" hx-target="#modal-content">Add Person</button>
</div>"##
                .to_string(),
        ),
    }
}
pub(super) async fn handle_people_cards(
    State(state): State<Arc<PeopleState>>,
    Query(query): Query<PeopleQuery>,
) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result: Option<Vec<PersonCardRow>> = tokio::task::spawn_blocking(move || -> Option<Vec<PersonCardRow>> {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        let mut db_query = people::table
            .filter(people::branch_id.eq(branch_id))
            .filter(people::is_active.eq(true))
            .into_boxed();

        if let Some(ref dept) = query.department {
            db_query = db_query.filter(people::department.eq(dept));
        }

        db_query = db_query.order(people::first_name.asc());

        let limit = query.limit.unwrap_or(20);
        db_query = db_query.limit(limit);

        db_query
            .select((
                people::id,
                people::first_name,
                people::last_name,
                people::email,
                people::job_title,
                people::department,
                people::avatar_url,
                people::phone,
            ))
            .load::<PersonCardRow>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(persons) if !persons.is_empty() => {
            let mut html = String::from(r##"<div class="people-cards-grid">"##);

            for row in persons {
                let full_name = format!("{} {}", row.first_name, row.last_name.unwrap_or_default());
                let email_str = row.email.unwrap_or_default();
                let title_str = row.job_title.unwrap_or_else(|| "Team Member".to_string());
                let dept_str = row.department.unwrap_or_default();
                let avatar = row.avatar_url.unwrap_or_else(|| "/assets/default-avatar.png".to_string());
                let phone_str = row.phone.unwrap_or_default();
                let id = row.id;

                html.push_str(&format!(
                    r##"<div class="person-card" data-id="{id}" hx-get="/api/ui/people/{id}" hx-target="#person-detail">
<div class="card-avatar">
<img src="{}" alt="{}" class="avatar-lg" />
</div>
<div class="card-info">
<h4 class="card-name">{}</h4>
<p class="card-title">{}</p>
<p class="card-department">{}</p>
</div>
<div class="card-contact">
<span class="card-email">{}</span>
<span class="card-phone">{}</span>
</div>
</div>"##,
                    html_escape(&avatar),
                    html_escape(&full_name),
                    html_escape(&full_name),
                    html_escape(&title_str),
                    html_escape(&dept_str),
                    html_escape(&email_str),
                    html_escape(&phone_str),
                ));
            }

            html.push_str("</div>");
            Html(html)
        }
        _ => Html(
            r##"<div class="empty-state">
<div class="empty-icon">👥</div>
<p>No people in directory</p>
</div>"##
                .to_string(),
        ),
    }
}
pub(super) async fn handle_people_count(State(state): State<Arc<PeopleState>>) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result: Option<i64> = tokio::task::spawn_blocking(move || -> Option<i64> {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        people::table
            .filter(people::branch_id.eq(branch_id))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(format!("{}", result.unwrap_or(0)))
}
pub(super) async fn handle_active_count(State(state): State<Arc<PeopleState>>) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result: Option<i64> = tokio::task::spawn_blocking(move || -> Option<i64> {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        people::table
            .filter(people::branch_id.eq(branch_id))
            .filter(people::is_active.eq(true))
            .count()
            .get_result::<i64>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    Html(format!("{}", result.unwrap_or(0)))
}
