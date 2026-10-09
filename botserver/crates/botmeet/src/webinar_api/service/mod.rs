pub(crate) use chrono::{DateTime, Utc};
pub(crate) use diesel::prelude::*;
pub(crate) use diesel::sql_types::{BigInt, Bool, Integer, Nullable, Text, Timestamptz, Uuid as DieselUuid};
pub(crate) use log::info;
pub(crate) use std::collections::HashMap;
pub(crate) use std::sync::Arc;
pub(crate) use tokio::sync::broadcast;
pub(crate) use uuid::Uuid;
pub(crate) use super::constants::{QA_QUESTION_MAX_LENGTH, MAX_RAISED_HANDS_VISIBLE};
pub(crate) use super::error::WebinarError;
pub(crate) use super::types::{ CreateWebinarRequest, ParticipantRole, ParticipantStatus, QAQuestion, QuestionStatus, RegisterRequest, RegistrationStatus, Webinar, WebinarParticipant, WebinarRegistration, WebinarSettings, WebinarStatus, WebinarEventType, };

#[derive(QueryableByName)]
struct WebinarRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    organization_id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    meeting_id: Uuid,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Nullable<Text>)]
    description: Option<String>,
    #[diesel(sql_type = Timestamptz)]
    scheduled_start: DateTime<Utc>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    scheduled_end: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    actual_start: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    actual_end: Option<DateTime<Utc>>,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Text)]
    settings_json: String,
    #[diesel(sql_type = Bool)]
    registration_required: bool,
    #[diesel(sql_type = Nullable<Text>)]
    registration_url: Option<String>,
    #[diesel(sql_type = DieselUuid)]
    host_id: Uuid,
    #[diesel(sql_type = Timestamptz)]
    created_at: DateTime<Utc>,
    #[diesel(sql_type = Timestamptz)]
    updated_at: DateTime<Utc>,
}

#[derive(QueryableByName)]
struct ParticipantRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    webinar_id: Uuid,
    #[diesel(sql_type = Nullable<DieselUuid>)]
    user_id: Option<Uuid>,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Nullable<Text>)]
    email: Option<String>,
    #[diesel(sql_type = Text)]
    role: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Bool)]
    hand_raised: bool,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    hand_raised_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Bool)]
    is_speaking: bool,
    #[diesel(sql_type = Bool)]
    video_enabled: bool,
    #[diesel(sql_type = Bool)]
    audio_enabled: bool,
    #[diesel(sql_type = Bool)]
    screen_sharing: bool,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    joined_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    left_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Nullable<Text>)]
    registration_data: Option<String>,
}

#[derive(QueryableByName)]
struct QuestionRow {
    #[diesel(sql_type = DieselUuid)]
    id: Uuid,
    #[diesel(sql_type = DieselUuid)]
    webinar_id: Uuid,
    #[diesel(sql_type = Nullable<DieselUuid>)]
    asker_id: Option<Uuid>,
    #[diesel(sql_type = Text)]
    asker_name: String,
    #[diesel(sql_type = Bool)]
    is_anonymous: bool,
    #[diesel(sql_type = Text)]
    question: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Integer)]
    upvotes: i32,
    #[diesel(sql_type = Nullable<Text>)]
    upvoted_by: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    answer: Option<String>,
    #[diesel(sql_type = Nullable<DieselUuid>)]
    answered_by: Option<Uuid>,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    answered_at: Option<DateTime<Utc>>,
    #[diesel(sql_type = Bool)]
    is_pinned: bool,
    #[diesel(sql_type = Bool)]
    is_highlighted: bool,
    #[diesel(sql_type = Timestamptz)]
    created_at: DateTime<Utc>,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

pub struct WebinarService {
    pool: Arc<diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<diesel::PgConnection>>>,
    event_sender: broadcast::Sender<super::types::WebinarEvent>,
}

mod part1;
mod part2;
mod part3;
