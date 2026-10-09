use super::*;

pub async fn list_people(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Person>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    let mut q = people::table
        .filter(people::branch_id.eq(branch_id))
        .into_boxed();

    if let Some(is_active) = query.is_active {
        q = q.filter(people::is_active.eq(is_active));
    }

    if let Some(department) = query.department {
        q = q.filter(people::department.eq(department));
    }

    if let Some(manager_id) = query.manager_id {
        q = q.filter(people::manager_id.eq(manager_id));
    }

    if let Some(search) = query.search {
        let pattern = format!("%{search}%");
        q = q.filter(
            people::first_name
                .ilike(pattern.clone())
                .or(people::last_name.ilike(pattern.clone()))
                .or(people::email.ilike(pattern.clone()))
                .or(people::job_title.ilike(pattern)),
        );
    }

    let persons: Vec<Person> = q
        .order((people::last_name.asc(), people::first_name.asc()))
        .limit(limit)
        .offset(offset)
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(persons))
}
pub async fn get_person(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<PersonWithDetails>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let person: Person = people::table
        .filter(people::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Person not found".to_string()))?;

    let manager: Option<Person> = person
        .manager_id
        .and_then(|mid| {
            people::table
                .filter(people::id.eq(mid))
                .first(&mut conn)
                .ok()
        });

    let direct_reports: Vec<Person> = people::table
        .filter(people::manager_id.eq(id))
        .filter(people::is_active.eq(true))
        .order(people::first_name.asc())
        .load(&mut conn)
        .unwrap_or_default();

    let team_ids: Vec<Uuid> = people_team_members::table
        .filter(people_team_members::person_id.eq(id))
        .select(people_team_members::team_id)
        .load(&mut conn)
        .unwrap_or_default();

    let teams: Vec<Team> = if team_ids.is_empty() {
        vec![]
    } else {
        people_teams::table
            .filter(people_teams::id.eq_any(&team_ids))
            .load(&mut conn)
            .unwrap_or_default()
    };

    let skill_ids: Vec<Uuid> = people_person_skills::table
        .filter(people_person_skills::person_id.eq(id))
        .select(people_person_skills::skill_id)
        .load(&mut conn)
        .unwrap_or_default();

    let skills: Vec<Skill> = if skill_ids.is_empty() {
        vec![]
    } else {
        people_skills::table
            .filter(people_skills::id.eq_any(&skill_ids))
            .load(&mut conn)
            .unwrap_or_default()
    };

    Ok(Json(PersonWithDetails {
        person,
        manager,
        direct_reports,
        teams,
        skills,
    }))
}
pub async fn update_person(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePersonRequest>,
) -> Result<Json<Person>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let now = Utc::now();

    diesel::update(people::table.filter(people::id.eq(id)))
        .set(people::updated_at.eq(now))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;

    if let Some(first_name) = req.first_name {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::first_name.eq(first_name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(last_name) = req.last_name {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::last_name.eq(last_name))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(email) = req.email {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::email.eq(email))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(job_title) = req.job_title {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::job_title.eq(job_title))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(department) = req.department {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::department.eq(department))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(manager_id) = req.manager_id {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::manager_id.eq(Some(manager_id)))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(is_active) = req.is_active {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::is_active.eq(is_active))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    if let Some(skills) = req.skills {
        diesel::update(people::table.filter(people::id.eq(id)))
            .set(people::skills.eq(skills))
            .execute(&mut conn)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Update error: {e}")))?;
    }

    let person: Person = people::table
        .filter(people::id.eq(id))
        .first(&mut conn)
        .map_err(|_| (StatusCode::NOT_FOUND, "Person not found".to_string()))?;

    Ok(Json(person))
}
pub async fn delete_person(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    diesel::delete(people::table.filter(people::id.eq(id)))
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {e}")))?;

    Ok(StatusCode::NO_CONTENT)
}
pub async fn get_direct_reports(
    State(state): State<Arc<PeopleState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Person>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let reports: Vec<Person> = people::table
        .filter(people::manager_id.eq(id))
        .filter(people::is_active.eq(true))
        .order(people::first_name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(reports))
}
pub async fn create_team(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
    Json(req): Json<CreateTeamRequest>,
) -> Result<Json<Team>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let team = Team {
        id,
        branch_id,
        name: req.name,
        description: req.description,
        leader_id: req.leader_id,
        parent_team_id: req.parent_team_id,
        color: req.color,
        icon: req.icon,
        is_active: true,
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(people_teams::table)
        .values(&team)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(team))
}
pub async fn list_teams(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<Team>>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn).unwrap_or_else(|| get_bot_context(&state));

    let teams: Vec<Team> = people_teams::table
        .filter(people_teams::branch_id.eq(branch_id))
        .filter(people_teams::is_active.eq(true))
        .order(people_teams::name.asc())
        .load(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Query error: {e}")))?;

    Ok(Json(teams))
}
