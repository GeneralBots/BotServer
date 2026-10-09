use super::*;

pub(super) async fn handle_time_off_list(State(state): State<Arc<PeopleState>>) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        people_time_off::table
            .filter(people_time_off::branch_id.eq(branch_id))
            .filter(people_time_off::status.eq("pending"))
            .order(people_time_off::created_at.desc())
            .limit(20)
            .select((
                people_time_off::id,
                people_time_off::person_id,
                people_time_off::time_off_type,
                people_time_off::status,
                people_time_off::start_date,
                people_time_off::end_date,
                people_time_off::reason,
            ))
            .load::<(Uuid, Uuid, String, String, chrono::NaiveDate, chrono::NaiveDate, Option<String>)>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(requests) if !requests.is_empty() => {
            let mut html = String::from(r##"<div class="time-off-list">"##);

            for (id, _person_id, time_off_type, status, start_date, end_date, reason) in requests {
                let reason_str = reason.unwrap_or_default();
                let start_str = start_date.format("%b %d").to_string();
                let end_str = end_date.format("%b %d, %Y").to_string();

                html.push_str(&format!(
                    r##"<div class="time-off-item" data-id="{id}">
<div class="time-off-header">
<span class="time-off-type">{}</span>
<span class="time-off-status status-{}">{}</span>
</div>
<div class="time-off-dates">
<span>{} - {}</span>
</div>
<p class="time-off-reason">{}</p>
<div class="time-off-actions">
<button class="btn-sm btn-success" hx-put="/api/people/time-off/{id}/approve" hx-swap="none">Approve</button>
<button class="btn-sm btn-danger" hx-put="/api/people/time-off/{id}/reject" hx-swap="none">Reject</button>
</div>
</div>"##,
                    html_escape(&time_off_type),
                    status,
                    html_escape(&status),
                    start_str,
                    end_str,
                    html_escape(&reason_str),
                ));
            }

            html.push_str("</div>");
            Html(html)
        }
        _ => Html(
            r##"<div class="empty-state">
<p>No pending time-off requests</p>
</div>"##
                .to_string(),
        ),
    }
}
pub(super) async fn handle_people_search(
    State(state): State<Arc<PeopleState>>,
    Query(query): Query<SearchQuery>,
) -> Html<String> {
    let q = query.q.clone().unwrap_or_default();
    if q.is_empty() {
        return Html(String::new());
    }

    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();
    let search_term = format!("%{q}%");

    let result: Option<Vec<PersonSearchRow>> = tokio::task::spawn_blocking(move || -> Option<Vec<PersonSearchRow>> {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        people::table
            .filter(people::branch_id.eq(branch_id))
            .filter(
                people::first_name.ilike(&search_term)
                    .or(people::last_name.ilike(&search_term))
                    .or(people::email.ilike(&search_term))
                    .or(people::job_title.ilike(&search_term)),
            )
            .order(people::first_name.asc())
            .limit(20)
            .select((
                people::id,
                people::first_name,
                people::last_name,
                people::email,
                people::job_title,
                people::avatar_url,
            ))
            .load::<PersonSearchRow>(&mut conn)
            .ok()
    })
    .await
    .ok()
    .flatten();

    match result {
        Some(persons) if !persons.is_empty() => {
            let mut html = String::from(r##"<div class="search-results">"##);

            for row in persons {
                let full_name = format!("{} {}", row.first_name, row.last_name.unwrap_or_default());
                let email_str: String = row.email.unwrap_or_default();
                let title_str: String = row.job_title.unwrap_or_default();
                let avatar: String = row.avatar_url.unwrap_or_else(|| "/assets/default-avatar.png".to_string());
                let id = row.id;

                html.push_str(&format!(
                    r##"<div class="search-result-item" hx-get="/api/ui/people/{id}" hx-target="#person-detail">
<img src="{}" class="avatar-sm" alt="" />
<div class="result-info">
<span class="result-name">{}</span>
<span class="result-title">{}</span>
<span class="result-email">{}</span>
</div>
</div>"##,
                    html_escape(&avatar),
                    html_escape(&full_name),
                    html_escape(&title_str),
                    html_escape(&email_str),
                ));
            }

            html.push_str("</div>");
            Html(html)
        }
        _ => Html(format!(
            r##"<div class="search-results-empty">
<p>No results for "{}"</p>
</div>"##,
            html_escape(&q),
        )),
    }
}
pub(super) async fn handle_people_stats(State(state): State<Arc<PeopleState>>) -> Html<String> {
    let pool = state.pool.clone();
    let get_default_bot = state.get_default_bot.clone();

    let result = tokio::task::spawn_blocking(move || -> Option<(i64, i64, i64, i64, i64)> {
        let mut conn = pool.get().ok()?;
        let branch_id = get_default_bot(&mut conn);

        let total: i64 = people::table
            .filter(people::branch_id.eq(branch_id))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let active: i64 = people::table
            .filter(people::branch_id.eq(branch_id))
            .filter(people::is_active.eq(true))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let departments: i64 = people_departments::table
            .filter(people_departments::branch_id.eq(branch_id))
            .filter(people_departments::is_active.eq(true))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let teams: i64 = people_teams::table
            .filter(people_teams::branch_id.eq(branch_id))
            .filter(people_teams::is_active.eq(true))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        let pending_time_off: i64 = people_time_off::table
            .filter(people_time_off::branch_id.eq(branch_id))
            .filter(people_time_off::status.eq("pending"))
            .count()
            .get_result(&mut conn)
            .unwrap_or(0);

        Some((total, active, departments, teams, pending_time_off))
    })
    .await
    .ok()
    .flatten();

    match result {
        Some((total, active, departments, teams, pending_time_off)) => Html(format!(
            r##"<div class="stats-grid">
<div class="stat-card">
<span class="stat-value">{total}</span>
<span class="stat-label">Total People</span>
</div>
<div class="stat-card stat-success">
<span class="stat-value">{active}</span>
<span class="stat-label">Active</span>
</div>
<div class="stat-card stat-info">
<span class="stat-value">{departments}</span>
<span class="stat-label">Departments</span>
</div>
<div class="stat-card stat-primary">
<span class="stat-value">{teams}</span>
<span class="stat-label">Teams</span>
</div>
<div class="stat-card stat-warning">
<span class="stat-value">{pending_time_off}</span>
<span class="stat-label">Pending Time Off</span>
</div>
</div>"##
        )),
        None => Html(
            r##"<div class="stats-grid"><div class="stat-card"><span class="stat-value">-</span></div></div>"##.to_string(),
        ),
    }
}
pub(super) async fn handle_new_person_form() -> Html<String> {
    Html(
        r##"<div class="modal-header">
<h3>Add New Person</h3>
<button class="btn-close" onclick="closeModal()">&times;</button>
</div>
<form class="person-form" hx-post="/api/people" hx-swap="none">
<div class="form-row">
<div class="form-group">
<label>First Name *</label>
<input type="text" name="first_name" required />
</div>
<div class="form-group">
<label>Last Name</label>
<input type="text" name="last_name" />
</div>
</div>
<div class="form-group">
<label>Email</label>
<input type="email" name="email" />
</div>
<div class="form-row">
<div class="form-group">
<label>Phone</label>
<input type="tel" name="phone" />
</div>
<div class="form-group">
<label>Mobile</label>
<input type="tel" name="mobile" />
</div>
</div>
<div class="form-group">
<label>Job Title</label>
<input type="text" name="job_title" />
</div>
<div class="form-group">
<label>Department</label>
<input type="text" name="department" />
</div>
<div class="form-group">
<label>Office Location</label>
<input type="text" name="office_location" />
</div>
<div class="form-group">
<label>Hire Date</label>
<input type="date" name="hire_date" />
</div>
<div class="form-group">
<label>Bio</label>
<textarea name="bio" rows="3" placeholder="Short biography"></textarea>
</div>
<div class="form-actions">
<button type="button" class="btn btn-secondary" onclick="closeModal()">Cancel</button>
<button type="submit" class="btn btn-primary">Add Person</button>
</div>
</form>"##
            .to_string(),
    )
}
