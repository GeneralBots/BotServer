use super::*;

pub use botlib::db_pool::DbPool;
pub type GetDefaultBotFn = Arc<dyn Fn(&mut diesel::PgConnection) -> Uuid + Send + Sync>;
#[derive(Clone)]
pub struct PeopleState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}

// Canonical definition lives in `botschema` (#1497).
pub use botschema::people::people;

pub use botschema::people_teams;

pub use botschema::people_team_members;

pub use botschema::people_org_chart;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::people_departments;

pub use botschema::people_skills;

pub use botschema::people_person_skills;

pub use botschema::people_time_off;

// `people`, `people_team_members` and `people_person_skills` are canonical in
// `botschema`, whose `joinable!`/`allow_tables_to_appear_in_same_query!` rules
// already cover them (#1497); repeating them between two foreign types breaks
// the orphan rule (E0117).
// Canonical definitions live in `botschema` (#1497).
pub use botschema::people::hr_clock_entries;
pub use botschema::people::hr_work_periods;
pub use botschema::people::hr_overtime_rules;
pub use botschema::people::hr_schedules;
pub use botschema::people::hr_holidays;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = people)]
pub struct Person {
    pub id: Uuid,
    pub org_id: Uuid,
    pub bot_id: Uuid,
    pub branch_id: Uuid,
    pub user_id: Option<Uuid>,
    pub first_name: String,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub mobile: Option<String>,
    pub job_title: Option<String>,
    pub department: Option<String>,
    pub manager_id: Option<Uuid>,
    pub office_location: Option<String>,
    pub hire_date: Option<NaiveDate>,
    pub birthday: Option<NaiveDate>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub skills: Vec<Option<String>>,
    pub social_links: serde_json::Value,
    pub custom_fields: serde_json::Value,
    pub timezone: Option<String>,
    pub locale: Option<String>,
    pub is_active: bool,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable, AsChangeset)]
#[diesel(table_name = people_teams)]
pub struct Team {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub leader_id: Option<Uuid>,
    pub parent_team_id: Option<Uuid>,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Insertable)]
#[diesel(table_name = people_team_members)]
pub struct TeamMember {
    pub id: Uuid,
    pub team_id: Uuid,
    pub person_id: Uuid,
    pub role: Option<String>,
    pub is_primary: bool,
    pub joined_at: DateTime<Utc>,
}
