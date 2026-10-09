use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = people_org_chart)]
pub struct OrgChartEntry {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub person_id: Uuid,
    pub reports_to_id: Option<Uuid>,
    pub position_title: Option<String>,
    pub position_level: i32,
    pub position_order: i32,
    pub effective_from: Option<NaiveDate>,
    pub effective_until: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = people_departments)]
pub struct Department {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub code: Option<String>,
    pub parent_id: Option<Uuid>,
    pub head_id: Option<Uuid>,
    pub cost_center: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = people_skills)]
pub struct Skill {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = people_person_skills)]
pub struct PersonSkill {
    pub id: Uuid,
    pub person_id: Uuid,
    pub skill_id: Uuid,
    pub proficiency_level: i32,
    pub years_experience: Option<BigDecimal>,
    pub verified_by: Option<Uuid>,
    pub verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = people_time_off)]
pub struct TimeOff {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub person_id: Uuid,
    pub time_off_type: String,
    pub status: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub hours_requested: Option<BigDecimal>,
    pub reason: Option<String>,
    pub approved_by: Option<Uuid>,
    pub approved_at: Option<DateTime<Utc>>,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Deserialize)]
pub struct CreatePersonRequest {
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub manager_id: Option<Uuid>,
    pub office_location: Option<String>,
    pub hire_date: Option<String>,
    pub birthday: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub skills: Option<Vec<String>>,
    pub timezone: Option<String>,
    pub locale: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct UpdatePersonRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub manager_id: Option<Uuid>,
    pub office_location: Option<String>,
    pub bio: Option<String>,
    pub skills: Option<Vec<String>>,
    pub is_active: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
    pub leader_id: Option<Uuid>,
    pub parent_team_id: Option<Uuid>,
    pub color: Option<String>,
    pub icon: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct AddTeamMemberRequest {
    pub person_id: Uuid,
    pub role: Option<String>,
    pub is_primary: Option<bool>,
}
#[derive(Debug, Deserialize)]
pub struct CreateDepartmentRequest {
    pub name: String,
    pub description: Option<String>,
    pub code: Option<String>,
    pub parent_id: Option<Uuid>,
    pub head_id: Option<Uuid>,
    pub cost_center: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct CreateSkillRequest {
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct AddPersonSkillRequest {
    pub skill_id: Uuid,
    pub proficiency_level: Option<i32>,
    pub years_experience: Option<f64>,
}
#[derive(Debug, Deserialize)]
pub struct CreateTimeOffRequest {
    pub person_id: Uuid,
    pub time_off_type: String,
    pub start_date: String,
    pub end_date: String,
    pub hours_requested: Option<f64>,
    pub reason: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct ApproveTimeOffRequest {
    pub approved: bool,
    pub notes: Option<String>,
}
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub search: Option<String>,
    pub department: Option<String>,
    pub team_id: Option<Uuid>,
    pub manager_id: Option<Uuid>,
    pub is_active: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
#[derive(Debug, Serialize)]
pub struct PeopleStats {
    pub total_people: i64,
    pub active_people: i64,
    pub total_teams: i64,
    pub total_departments: i64,
    pub pending_time_off: i64,
    pub new_hires_this_month: i64,
}
#[derive(Debug, Serialize)]
pub struct PersonWithDetails {
    pub person: Person,
    pub manager: Option<Person>,
    pub direct_reports: Vec<Person>,
    pub teams: Vec<Team>,
    pub skills: Vec<Skill>,
}
#[derive(Debug, Serialize)]
pub struct TeamWithMembers {
    pub team: Team,
    pub members: Vec<Person>,
    pub leader: Option<Person>,
}
#[derive(Debug, Serialize)]
pub struct OrgChartNode {
    pub person: Person,
    pub position_title: Option<String>,
    pub position_level: i32,
    pub reports: Vec<OrgChartNode>,
}
pub(super) fn get_bot_context(state: &PeopleState) -> Uuid {
    let Ok(mut conn) = state.pool.get() else {
        return Uuid::nil();
    };
    (state.get_default_bot)(&mut conn)
}
pub(super) fn bd(val: f64) -> BigDecimal {
    use std::str::FromStr;
    BigDecimal::from_str(&val.to_string()).unwrap_or_else(|_| BigDecimal::from(0))
}
pub async fn create_person(
    State(state): State<Arc<PeopleState>>,
    headers: HeaderMap,
    Json(req): Json<CreatePersonRequest>,
) -> Result<Json<Person>, (StatusCode, String)> {
    let mut conn = state.pool.get().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB error: {e}"))
    })?;

    let branch_id = crate::scope::branch_from_jwt(&headers, &mut conn)
        .unwrap_or_else(|| get_bot_context(&state));
    let id = Uuid::new_v4();
    let now = Utc::now();

    let hire_date = req
        .hire_date
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let birthday = req
        .birthday
        .and_then(|d| NaiveDate::parse_from_str(&d, "%Y-%m-%d").ok());

    let person = Person {
        id,
        org_id: state.org_for_branch(branch_id),
        bot_id: state.bot_for_branch(branch_id),
        branch_id,
        user_id: None,
        first_name: req.first_name,
        last_name: req.last_name,
        email: req.email,
        phone: req.phone,
        mobile: req.mobile,
        job_title: req.job_title,
        department: req.department,
        manager_id: req.manager_id,
        office_location: req.office_location,
        hire_date,
        birthday,
        avatar_url: req.avatar_url,
        bio: req.bio,
        skills: req.skills.unwrap_or_default(),
        social_links: serde_json::json!({}),
        custom_fields: serde_json::json!({}),
        timezone: req.timezone,
        locale: req.locale,
        is_active: true,
        last_seen_at: None,
        created_at: now,
        updated_at: now,
    };

    diesel::insert_into(people::table)
        .values(&person)
        .execute(&mut conn)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Insert error: {e}")))?;

    Ok(Json(person))
}
