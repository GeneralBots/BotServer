use super::*;

pub async fn get_team(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<TeamWithMembers>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let team: Team = people_teams::table
        .filter(people_teams::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Team not found".to_string()))?;

    let member_ids: Vec<Uuid> = people_team_members::table
        .filter(people_team_members::team_id.eq(id))
        .select(people_team_members::person_id)
        .load(&mut conn)
        .unwrap_or_default();

    let members: Vec<Person> = if member_ids.is_empty() {
        vec![]
    } else {
        people::table
            .filter(people::id.eq_any(&member_ids))
            .load(&mut conn)
            .unwrap_or_default()
    };

    let leader: Option<Person> = team
        .leader_id
        .and_then(|lid| {
            people::table
                .filter(people::id.eq(lid))
                .first(&mut conn)
                .ok()
        });

    Ok(Json(TeamWithMembers {
        team,
        members,
        leader,
    }))
}
pub async fn add_team_member(
    State(state): State<Arc<PeopleState>>,
    Path(team_id): Path<Uuid>,
    Json(req): Json<AddTeamMemberRequest>,
) -> Result<Json<TeamMember>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    let member = TeamMember {
        id,
        team_id,
        person_id: req.person_id,
        role: req.role,
        is_primary: req.is_primary.unwrap_or(false),
        joined_at: now,
    };

    diesel::insert_into(people_team_members::table)
        .values(&member)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(member))
}
pub async fn remove_team_member(
    State(state): State<Arc<PeopleState>>,
    Path((team_id, person_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(
        people_team_members::table
            .filter(people_team_members::team_id.eq(team_id))
            .filter(people_team_members::person_id.eq(person_id)),
    )
    .execute(&mut conn)
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}
pub async fn delete_team(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(people_teams::table.filter(people_teams::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}
pub async fn create_department(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
    Json(req): Json<CreateDepartmentRequest>,
) -> Result<Json<Department>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let (org_id, bot_id) = org_and_bot(&mut conn, branch_id);
    let department = Department {
        id,
        org_id,
        bot_id,
        name: req.name,
        description: req.description,
        code: req.code,
        parent_id: req.parent_id,
        head_id: req.head_id,
        cost_center: req.cost_center,
        is_active: true,
        created_at: now,
        updated_at: now,
        branch_id,
    };

    diesel::insert_into(people_departments::table)
        .values(&department)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(department))
}
pub async fn list_departments(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Department>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let depts: Vec<Department> = people_departments::table
        .filter(people_departments::branch_id.eq(branch_id))
        .filter(people_departments::is_active.eq(true))
        .order(people_departments::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(depts))
}
pub async fn list_skills(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Skill>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let skills: Vec<Skill> = people_skills::table
        .filter(people_skills::branch_id.eq(branch_id))
        .filter(people_skills::is_active.eq(true))
        .order(people_skills::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(skills))
}
pub async fn create_skill(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
    Json(req): Json<CreateSkillRequest>,
) -> Result<Json<Skill>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let (org_id, bot_id) = org_and_bot(&mut conn, branch_id);
    let skill = Skill {
        id,
        org_id,
        bot_id,
        name: req.name,
        category: req.category,
        description: req.description,
        is_active: true,
        created_at: now,
        branch_id,
    };

    diesel::insert_into(people_skills::table)
        .values(&skill)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(skill))
}
pub async fn add_person_skill(
    State(state): State<Arc<PeopleState>>,
    Path(person_id): Path<Uuid>,
    Json(req): Json<AddPersonSkillRequest>,
) -> Result<Json<PersonSkill>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let id = Uuid::new_v4();
    let now = Utc::now();

    let person_skill = PersonSkill {
        id,
        person_id,
        skill_id: req.skill_id,
        proficiency_level: req.proficiency_level.unwrap_or(1),
        years_experience: req.years_experience.map(bd),
        verified_by: None,
        verified_at: None,
        created_at: now,
    };

    diesel::insert_into(people_person_skills::table)
        .values(&person_skill)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(person_skill))
}
pub async fn create_time_off(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
    Json(req): Json<CreateTimeOffRequest>,
) -> Result<Json<TimeOff>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let start_date = NaiveDate::parse_from_str(&req.start_date, "%Y-%m-%d")
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid start_date".to_string()))?;

    let end_date = NaiveDate::parse_from_str(&req.end_date, "%Y-%m-%d")
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid end_date".to_string()))?;

    let (org_id, bot_id) = org_and_bot(&mut conn, branch_id);
    let time_off = TimeOff {
        id,
        org_id,
        bot_id,
        person_id: req.person_id,
        time_off_type: req.time_off_type,
        status: "pending".to_string(),
        start_date,
        end_date,
        hours_requested: req.hours_requested.map(bd),
        reason: req.reason,
        approved_by: None,
        approved_at: None,
        notes: None,
        created_at: now,
        updated_at: now,
        branch_id,
    };

    diesel::insert_into(people_time_off::table)
        .values(&time_off)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(time_off))
}
