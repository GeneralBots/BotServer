use super::*;

pub(super) async fn handle_person_detail(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
) -> Html<String> {
    let pool = state.pool.clone();

    let result: Option<PersonDetailRow> = tokio::task::spawn_blocking(move || -> Option<PersonDetailRow> {
        let mut conn = pool.get().ok()?;

        people::table
            .find(id)
            .select((
                people::id,
                people::first_name,
                people::last_name,
                people::email,
                people::phone,
                people::mobile,
                people::job_title,
                people::department,
                people::office_location,
                people::avatar_url,
                people::bio,
                people::hire_date,
                people::is_active,
                people::last_seen_at,
            ))
            .first::<PersonDetailRow>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(row) => {
            let full_name = format!("{} {}", row.first_name, row.last_name.unwrap_or_default());
            let email_str = row.email.unwrap_or_else(|| "-".to_string());
            let phone_str = row.phone.unwrap_or_else(|| "-".to_string());
            let mobile_str = row.mobile.unwrap_or_else(|| "-".to_string());
            let title_str = row.job_title.unwrap_or_else(|| "-".to_string());
            let dept_str = row.department.unwrap_or_else(|| "-".to_string());
            let office_str = row.office_location.unwrap_or_else(|| "-".to_string());
            let avatar = row.avatar_url.unwrap_or_else(|| "/assets/default-avatar.png".to_string());
            let bio_str = row.bio.unwrap_or_else(|| "No bio available".to_string());
            let hire_str = row.hire_date.map(|d| d.format("%B %d, %Y").to_string()).unwrap_or_else(|| "-".to_string());
            let last_seen_str = row.last_seen_at.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_else(|| "Never".to_string());
            let is_active = row.is_active;
            let id = row.id;
            let status_class = if is_active { "status-active" } else { "status-inactive" };
            let status_text = if is_active { "Active" } else { "Inactive" };

            Html(format!(
                r##"<div class="person-detail-card">
<div class="detail-header">
<img src="{}" alt="{}" class="avatar-xl" />
<div class="header-info">
<h2>{}</h2>
<p class="job-title">{}</p>
<span class="{}">{}</span>
</div>
</div>
<div class="detail-section">
<h4>Contact Information</h4>
<div class="detail-grid">
<div class="detail-item">
<label>Email</label>
<span><a href="mailto:{}">{}</a></span>
</div>
<div class="detail-item">
<label>Phone</label>
<span>{}</span>
</div>
<div class="detail-item">
<label>Mobile</label>
<span>{}</span>
</div>
<div class="detail-item">
<label>Office</label>
<span>{}</span>
</div>
</div>
</div>
<div class="detail-section">
<h4>Work Information</h4>
<div class="detail-grid">
<div class="detail-item">
<label>Department</label>
<span>{}</span>
</div>
<div class="detail-item">
<label>Hire Date</label>
<span>{}</span>
</div>
<div class="detail-item">
<label>Last Seen</label>
<span>{}</span>
</div>
</div>
</div>
<div class="detail-section">
<h4>Bio</h4>
<p class="bio-text">{}</p>
</div>
<div class="detail-actions">
<button class="btn btn-primary" hx-get="/api/people/{}/edit" hx-target="#modal-content">Edit</button>
<button class="btn btn-secondary" hx-get="/api/ui/people/{}/reports" hx-target="#reports-panel">View Reports</button>
<button class="btn btn-danger" hx-delete="/api/people/{}" hx-swap="none" hx-confirm="Deactivate this person?">Deactivate</button>
</div>
</div>"##,
                html_escape(&avatar),
                html_escape(&full_name),
                html_escape(&full_name),
                html_escape(&title_str),
                status_class,
                status_text,
                html_escape(&email_str),
                html_escape(&email_str),
                html_escape(&phone_str),
                html_escape(&mobile_str),
                html_escape(&office_str),
                html_escape(&dept_str),
                hire_str,
                last_seen_str,
                html_escape(&bio_str),
                id,
                id,
                id,
            ))
        }
        None => Html(
            r##"<div class="empty-state">
<p>Person not found</p>
</div>"##
                .to_string(),
        ),
    }
}
pub(super) async fn handle_departments_list(State(state): State<Arc<PeopleState>>) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        people_departments::table
            .filter(people_departments::branch_id.eq(branch_id))
            .filter(people_departments::is_active.eq(true))
            .order(people_departments::name.asc())
            .select((
                people_departments::id,
                people_departments::name,
                people_departments::description,
                people_departments::code,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(depts) if !depts.is_empty() => {
            let mut html = String::from(r##"<div class="departments-list">"##);

            for (id, name, description, code) in depts {
                let desc_str = description.unwrap_or_default();
                let code_str = code.unwrap_or_default();

                html.push_str(&format!(
                    r##"<div class="department-item" data-id="{id}">
<div class="dept-header">
<span class="dept-name">{}</span>
<span class="dept-code">{}</span>
</div>
<p class="dept-description">{}</p>
<button class="btn-sm" hx-get="/api/ui/people?department={}" hx-target="#people-list">View Members</button>
</div>"##,
                    html_escape(&name),
                    html_escape(&code_str),
                    html_escape(&desc_str),
                    html_escape(&name),
                ));
            }

            html.push_str("</div>");
            Html(html)
        }
        _ => Html(
            r##"<div class="empty-state">
<p>No departments yet</p>
</div>"##
                .to_string(),
        ),
    }
}
pub(super) async fn handle_teams_list(State(state): State<Arc<PeopleState>>) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        people_teams::table
            .filter(people_teams::branch_id.eq(branch_id))
            .filter(people_teams::is_active.eq(true))
            .order(people_teams::name.asc())
            .select((
                people_teams::id,
                people_teams::name,
                people_teams::description,
                people_teams::color,
            ))
            .load::<(Uuid, String, Option<String>, Option<String>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(teams) if !teams.is_empty() => {
            let mut html = String::from(r##"<div class="teams-list">"##);

            for (id, name, description, color) in teams {
                let desc_str = description.unwrap_or_default();
                let team_color = color.unwrap_or_else(|| "#3b82f6".to_string());

                html.push_str(&format!(
                    r##"<div class="team-item" data-id="{id}" style="border-left: 4px solid {};">
<div class="team-header">
<span class="team-name">{}</span>
</div>
<p class="team-description">{}</p>
<button class="btn-sm" hx-get="/api/ui/people?team_id={id}" hx-target="#people-list">View Members</button>
</div>"##,
                    team_color,
                    html_escape(&name),
                    html_escape(&desc_str),
                ));
            }

            html.push_str("</div>");
            Html(html)
        }
        _ => Html(
            r##"<div class="empty-state">
<p>No teams yet</p>
</div>"##
                .to_string(),
        ),
    }
}
