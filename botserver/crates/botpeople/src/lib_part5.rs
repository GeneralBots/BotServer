use super::*;

pub async fn list_time_off(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<TimeOff>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let time_offs: Vec<TimeOff> = people_time_off::table
        .filter(people_time_off::branch_id.eq(branch_id))
        .order(people_time_off::start_date.desc())
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(time_offs))
}
pub async fn approve_time_off(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<ApproveTimeOffRequest>,
) -> Result<Json<TimeOff>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();
    let status = if req.approved { "approved" } else { "rejected" };

    diesel::update(people_time_off::table.filter(people_time_off::id.eq(id)))
        .set((
            people_time_off::status.eq(status),
            people_time_off::approved_at.eq(Some(now)),
            people_time_off::notes.eq(req.notes),
            people_time_off::updated_at.eq(now),
        ))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    let time_off: TimeOff = people_time_off::table
        .filter(people_time_off::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Time off request not found".to_string()))?;

    Ok(Json(time_off))
}
pub async fn get_people_stats(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
) -> Result<Json<PeopleStats>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let total_people: i64 = people::table
        .filter(people::branch_id.eq(branch_id))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let active_people: i64 = people::table
        .filter(people::branch_id.eq(branch_id))
        .filter(people::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let total_teams: i64 = people_teams::table
        .filter(people_teams::branch_id.eq(branch_id))
        .filter(people_teams::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let total_departments: i64 = people_departments::table
        .filter(people_departments::branch_id.eq(branch_id))
        .filter(people_departments::is_active.eq(true))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let pending_time_off: i64 = people_time_off::table
        .filter(people_time_off::branch_id.eq(branch_id))
        .filter(people_time_off::status.eq("pending"))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let today = Utc::now().date_naive();
    let month_start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .unwrap_or(today);

    let new_hires_this_month: i64 = people::table
        .filter(people::branch_id.eq(branch_id))
        .filter(people::hire_date.ge(month_start))
        .count()
        .get_result(&mut conn)
        .unwrap_or(0);

    let stats = PeopleStats {
        total_people,
        active_people,
        total_teams,
        total_departments,
        pending_time_off,
        new_hires_this_month,
    };

    Ok(Json(stats))
}
impl PeopleState {
    /// Resolves the org owning the given branch (branch → org via branches).
    /// The org is the gborg tenant that owns the workspace; it is never
    /// conflated with the branch id.
    pub fn org_for_branch(&self, branch_id: Uuid) -> Uuid {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            org_id: Uuid,
        }
        let Ok(mut conn) = self.pool.get() else {
            return Uuid::nil();
        };
        diesel::sql_query("SELECT org_id FROM branches WHERE id = $1 LIMIT 1")
            .bind::<diesel::sql_types::Uuid, _>(branch_id)
            .get_result::<Row>(&mut conn)
            .map(|r| r.org_id)
            .unwrap_or_else(|_| Uuid::nil())
    }

    /// Resolves a bot id inside the given branch (the branch's default bot
    /// when flagged, otherwise any active bot in the branch).
    pub fn bot_for_branch(&self, branch_id: Uuid) -> Uuid {
        #[derive(diesel::QueryableByName)]
        struct Row {
            #[diesel(sql_type = diesel::sql_types::Uuid)]
            id: Uuid,
        }
        let Ok(mut conn) = self.pool.get() else {
            return Uuid::nil();
        };
        diesel::sql_query(
            "SELECT id FROM bots WHERE branch_id = $1 AND is_active = true \
             ORDER BY is_default_for_branch DESC LIMIT 1",
        )
        .bind::<diesel::sql_types::Uuid, _>(branch_id)
        .get_result::<Row>(&mut conn)
        .map(|r| r.id)
        .unwrap_or_else(|_| Uuid::nil())
    }
}
pub fn configure_people_routes() -> Router<Arc<PeopleState>> {
    Router::new()
        .route("/api/people", get(list_people).post(create_person))
        .route("/api/people/stats", get(get_people_stats))
        .route("/api/people/:id", get(get_person).put(update_person).delete(delete_person))
        .route("/api/people/:id/reports", get(get_direct_reports))
        .route("/api/people/:id/skills", post(add_person_skill))
        .route("/api/people/teams", get(list_teams).post(create_team))
        .route("/api/people/teams/:id", get(get_team).delete(delete_team))
        .route("/api/people/teams/:id/members", post(add_team_member))
        .route("/api/people/teams/:team_id/members/:person_id", delete(remove_team_member))
        .route("/api/people/departments", get(list_departments).post(create_department))
        .route("/api/people/skills", get(list_skills).post(create_skill))
        .route("/api/people/time-off", get(list_time_off).post(create_time_off))
        .route("/api/people/time-off/:id/approve", put(approve_time_off))
}
