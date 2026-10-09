use super::*;

pub type DbPool = diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>;
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

diesel::table! {
    people_teams (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        leader_id -> Nullable<Uuid>,
        parent_team_id -> Nullable<Uuid>,
        color -> Nullable<Varchar>,
        icon -> Nullable<Varchar>,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    people_team_members (id) {
        id -> Uuid,
        team_id -> Uuid,
        person_id -> Uuid,
        role -> Nullable<Varchar>,
        is_primary -> Bool,
        joined_at -> Timestamptz,
    }
}

diesel::table! {
    people_org_chart (id) {
        id -> Uuid,
        branch_id -> Uuid,
        person_id -> Uuid,
        reports_to_id -> Nullable<Uuid>,
        position_title -> Nullable<Varchar>,
        position_level -> Int4,
        position_order -> Int4,
        effective_from -> Nullable<Date>,
        effective_until -> Nullable<Date>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    people_departments (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        description -> Nullable<Text>,
        code -> Nullable<Varchar>,
        parent_id -> Nullable<Uuid>,
        head_id -> Nullable<Uuid>,
        cost_center -> Nullable<Varchar>,
        is_active -> Bool,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    people_skills (id) {
        id -> Uuid,
        branch_id -> Uuid,
        name -> Varchar,
        category -> Nullable<Varchar>,
        description -> Nullable<Text>,
        is_active -> Bool,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    people_person_skills (id) {
        id -> Uuid,
        person_id -> Uuid,
        skill_id -> Uuid,
        proficiency_level -> Int4,
        years_experience -> Nullable<Numeric>,
        verified_by -> Nullable<Uuid>,
        verified_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    people_time_off (id) {
        id -> Uuid,
        branch_id -> Uuid,
        person_id -> Uuid,
        time_off_type -> Varchar,
        status -> Varchar,
        start_date -> Date,
        end_date -> Date,
        hours_requested -> Nullable<Numeric>,
        reason -> Nullable<Text>,
        approved_by -> Nullable<Uuid>,
        approved_at -> Nullable<Timestamptz>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::joinable!(people_team_members -> people_teams (team_id));
diesel::joinable!(people_team_members -> people (person_id));
diesel::joinable!(people_person_skills -> people (person_id));
diesel::joinable!(people_person_skills -> people_skills (skill_id));
diesel::allow_tables_to_appear_in_same_query!(
    people,
    people_teams,
    people_team_members,
    people_org_chart,
    people_departments,
    people_skills,
    people_person_skills,
    people_time_off,
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
