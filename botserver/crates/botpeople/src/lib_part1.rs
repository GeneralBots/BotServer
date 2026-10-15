use super::*;

pub use botlib::db_pool::DbPool;
pub type GetDefaultBotFn = Arc<dyn Fn(&mut diesel::PgConnection) -> Uuid + Send + Sync>;
#[derive(Clone)]
pub struct PeopleState {
    pub pool: Arc<DbPool>,
    pub get_default_bot: GetDefaultBotFn,
}

diesel::table! {
    people (id) {
        id -> Uuid,
        org_id -> Uuid,
        bot_id -> Uuid,
        branch_id -> Uuid,
        user_id -> Nullable<Uuid>,
        first_name -> Varchar,
        last_name -> Nullable<Varchar>,
        email -> Nullable<Varchar>,
        phone -> Nullable<Varchar>,
        mobile -> Nullable<Varchar>,
        job_title -> Nullable<Varchar>,
        department -> Nullable<Varchar>,
        manager_id -> Nullable<Uuid>,
        office_location -> Nullable<Varchar>,
        hire_date -> Nullable<Date>,
        birthday -> Nullable<Date>,
        avatar_url -> Nullable<Text>,
        bio -> Nullable<Text>,
        skills -> Array<Text>,
        social_links -> Jsonb,
        custom_fields -> Jsonb,
        timezone -> Nullable<Varchar>,
        locale -> Nullable<Varchar>,
        is_active -> Bool,
        last_seen_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

pub use botschema::people_teams;

pub use botschema::people_team_members;

pub use botschema::people_org_chart;

// Canonical definition lives in `botschema` (#1497).
pub use botschema::people_departments;

pub use botschema::people_skills;

pub use botschema::people_person_skills;

pub use botschema::people_time_off;

diesel::joinable!(people_team_members -> people (person_id));
diesel::joinable!(people_person_skills -> people (person_id));
diesel::allow_tables_to_appear_in_same_query!(
    people,
    hr_clock_entries,
    hr_work_periods,
    hr_overtime_rules,
    hr_schedules,
    hr_holidays,
);

diesel::table! {
    hr_clock_entries (id) {
        id -> Uuid,
        branch_id -> Uuid,
        person_id -> Uuid,
        entry_type -> Varchar,
        timestamp -> Timestamptz,
        latitude -> Nullable<Numeric>,
        longitude -> Nullable<Numeric>,
        geofence_id -> Nullable<Uuid>,
        device_id -> Nullable<Varchar>,
        ip_address -> Nullable<Varchar>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_work_periods (id) {
        id -> Uuid,
        branch_id -> Uuid,
        person_id -> Uuid,
        period_start -> Date,
        period_end -> Date,
        hours_worked -> Numeric,
        hours_overtime -> Numeric,
        hours_night -> Numeric,
        absences -> Integer,
        status -> Varchar,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_overtime_rules (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        weekly_threshold_hours -> Numeric,
        daily_threshold_hours -> Nullable<Numeric>,
        overtime_multiplier -> Numeric,
        night_shift_multiplier -> Numeric,
        holiday_multiplier -> Numeric,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_schedules (id) {
        id -> Uuid,
        branch_id -> Uuid,
        person_id -> Uuid,
        weekday -> Integer,
        start_time -> Time,
        end_time -> Time,
        break_minutes -> Integer,
        effective_from -> Date,
        effective_until -> Nullable<Date>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    hr_holidays (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        holiday_date -> Date,
        is_recurring -> Bool,
        region -> Nullable<Varchar>,
        created_at -> Timestamptz,
    }
}
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
    pub skills: Vec<String>,
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
