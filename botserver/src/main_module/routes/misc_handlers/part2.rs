use super::*;

pub async fn handle_calendar_event_save(
    State(state): State<Arc<AppState>>,
    Form(payload): Form<SaveEventRequest>,
) -> impl IntoResponse {
    use diesel::prelude::*;

    let title = payload.title.unwrap_or_default();
    if title.trim().is_empty() {
        return Html("<div class=\"calendar-error\">Title is required</div>".to_string());
    }

    let id = Uuid::new_v4();
    let mut conn = match pool_conn(&state) {
        Ok(c) => c,
        Err(e) => return Html(format!("<div class=\"calendar-error\">{}</div>", html_escape(&e.1))),
    };
    let branch = branch_id_for(&mut conn);
    let bot = bot_id_for(&mut conn, branch);

    let start_str = payload.start.unwrap_or_default();
    let end_str = payload.end.unwrap_or_default();
    let parse = |s: &str| -> Option<chrono::DateTime<Utc>> {
        if s.is_empty() {
            return None;
        }
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
            .ok()
            .map(|dt| chrono::DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc))
    };
    let start = parse(&start_str).unwrap_or_else(Utc::now);
    let end = parse(&end_str).unwrap_or(start + chrono::Duration::hours(1));
    let all_day = matches!(payload.all_day.as_deref(), Some("on") | Some("true"));

    // All three scope columns are required: org_id, bot_id and branch_id.
    let insert = diesel::sql_query(
        "INSERT INTO calendar_events \
         (id, org_id, bot_id, branch_id, calendar_id, owner_id, title, description, location, \
          start_time, end_time, all_day, status, visibility, busy_status, reminders, attendees, \
          metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'confirmed', 'default', \
                 'busy', '[]', '[]', '{}')",
    )
    .bind::<diesel::sql_types::Uuid, _>(&id)
    .bind::<diesel::sql_types::Uuid, _>(&Uuid::nil())
    .bind::<diesel::sql_types::Uuid, _>(&bot)
    .bind::<diesel::sql_types::Uuid, _>(&branch)
    .bind::<diesel::sql_types::Uuid, _>(&Uuid::nil())
    .bind::<diesel::sql_types::Uuid, _>(&Uuid::nil())
    .bind::<diesel::sql_types::Text, _>(&title)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.description)
    .bind::<diesel::sql_types::Nullable<diesel::sql_types::Text>, _>(&payload.location)
    .bind::<diesel::sql_types::Timestamptz, _>(&start)
    .bind::<diesel::sql_types::Timestamptz, _>(&end)
    .bind::<diesel::sql_types::Bool, _>(all_day)
    .execute(&mut conn);

    // The result used to be discarded, so the form confirmed a save that had
    // failed. Surface the failure instead of reporting a phantom event.
    if let Err(e) = insert {
        log::error!("Failed to save calendar event '{title}': {e}");
        return Html(
            "<div class=\"calendar-error\">The event could not be saved. Please try again.</div>"
                .to_string(),
        );
    }

    Html(format!(
        r#"<div class="event-saved" data-event-id="{id}"><p>Event "{title}" saved</p></div>"#,
        id = id,
        title = html_escape(&title),
    ))
}

// ---------------------------------------------------------------------------
// Goals: /api/goals/objectives/new (form modal)
// ---------------------------------------------------------------------------
pub async fn handle_goals_objective_new() -> impl IntoResponse {
    Html(
        r##"<div class="goals-modal-content">
        <h3>New Objective</h3>
        <form hx-post="/api/goals/objectives" hx-target="#goals-list" hx-swap="afterbegin">
            <div class="form-group">
                <label>Objective title</label>
                <input type="text" name="title" required placeholder="e.g. Grow revenue 30%" />
            </div>
            <div class="form-group">
                <label>Period</label>
                <select name="period"><option value="Q1">Q1</option><option value="Q2">Q2</option><option value="Q3">Q3</option><option value="Q4">Q4</option><option value="year">Year</option></select>
            </div>
            <div class="form-group">
                <label>Description</label>
                <textarea name="description" rows="3" placeholder="Describe the objective"></textarea>
            </div>
            <button type="submit" class="btn-primary btn-sm">Create Objective</button>
        </form>
        </div>"##,
    )
}

// ---------------------------------------------------------------------------
// Autotask: /api/ui/autotask/create
// ---------------------------------------------------------------------------
