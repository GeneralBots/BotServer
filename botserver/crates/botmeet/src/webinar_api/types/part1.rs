use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Webinar {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub meeting_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub actual_start: Option<DateTime<Utc>>,
    pub actual_end: Option<DateTime<Utc>>,
    pub status: WebinarStatus,
    pub settings: WebinarSettings,
    pub registration_required: bool,
    pub registration_url: Option<String>,
    pub host_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebinarStatus {
    Draft,
    Scheduled,
    Live,
    Paused,
    Ended,
    Cancelled,
}
impl std::fmt::Display for WebinarStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Scheduled => write!(f, "scheduled"),
            Self::Live => write!(f, "live"),
            Self::Paused => write!(f, "paused"),
            Self::Ended => write!(f, "ended"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarSettings {
    pub allow_attendee_video: bool,
    pub allow_attendee_audio: bool,
    pub allow_chat: bool,
    pub allow_qa: bool,
    pub allow_hand_raise: bool,
    pub allow_reactions: bool,
    pub moderated_qa: bool,
    pub anonymous_qa: bool,
    pub auto_record: bool,
    pub waiting_room_enabled: bool,
    pub max_attendees: u32,
    pub practice_session_enabled: bool,
    pub attendee_registration_fields: Vec<RegistrationField>,
    /// Enable automatic transcription during recording
    pub auto_transcribe: bool,
    /// Language for transcription (e.g., "en-US", "es-ES")
    pub transcription_language: Option<String>,
    /// Enable speaker identification in transcription
    pub transcription_speaker_identification: bool,
    /// Store recording in cloud storage
    pub cloud_recording: bool,
    /// Recording quality setting
    pub recording_quality: RecordingQuality,
}
/// Recording quality settings
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum RecordingQuality {
    #[default]
    Standard,  // 720p
    High,      // 1080p
    Ultra,     // 4K
    AudioOnly, // Audio only recording
}
impl std::fmt::Display for RecordingQuality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordingQuality::Standard => write!(f, "standard"),
            RecordingQuality::High => write!(f, "high"),
            RecordingQuality::Ultra => write!(f, "ultra"),
            RecordingQuality::AudioOnly => write!(f, "audio_only"),
        }
    }
}
impl Default for WebinarSettings {
    fn default() -> Self {
        Self {
            allow_attendee_video: false,
            allow_attendee_audio: false,
            allow_chat: true,
            allow_qa: true,
            allow_hand_raise: true,
            allow_reactions: true,
            moderated_qa: true,
            anonymous_qa: false,
            auto_record: false,
            waiting_room_enabled: true,
            max_attendees: MAX_WEBINAR_PARTICIPANTS as u32,
            practice_session_enabled: false,
            attendee_registration_fields: vec![
                RegistrationField::required("name"),
                RegistrationField::required("email"),
            ],
            auto_transcribe: true,
            transcription_language: Some("en-US".to_string()),
            transcription_speaker_identification: true,
            cloud_recording: true,
            recording_quality: RecordingQuality::default(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationField {
    pub name: String,
    pub field_type: FieldType,
    pub required: bool,
    pub options: Option<Vec<String>>,
}
impl RegistrationField {
    pub fn required(name: &str) -> Self {
        Self {
            name: name.to_string(),
            field_type: FieldType::Text,
            required: true,
            options: None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldType {
    Text,
    Email,
    Phone,
    Select,
    Checkbox,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRole {
    Host,
    CoHost,
    Presenter,
    Panelist,
    Attendee,
}
impl std::fmt::Display for ParticipantRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host => write!(f, "host"),
            Self::CoHost => write!(f, "co_host"),
            Self::Presenter => write!(f, "presenter"),
            Self::Panelist => write!(f, "panelist"),
            Self::Attendee => write!(f, "attendee"),
        }
    }
}
impl ParticipantRole {
    pub fn can_present(&self) -> bool {
        matches!(self, Self::Host | Self::CoHost | Self::Presenter | Self::Panelist)
    }

    pub fn can_manage(&self) -> bool {
        matches!(self, Self::Host | Self::CoHost)
    }

    pub fn can_speak(&self) -> bool {
        matches!(self, Self::Host | Self::CoHost | Self::Presenter | Self::Panelist)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarParticipant {
    pub id: Uuid,
    pub webinar_id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub email: Option<String>,
    pub role: ParticipantRole,
    pub status: ParticipantStatus,
    pub hand_raised: bool,
    pub hand_raised_at: Option<DateTime<Utc>>,
    pub is_speaking: bool,
    pub video_enabled: bool,
    pub audio_enabled: bool,
    pub screen_sharing: bool,
    pub joined_at: Option<DateTime<Utc>>,
    pub left_at: Option<DateTime<Utc>>,
    pub registration_data: Option<HashMap<String, String>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantStatus {
    Registered,
    InWaitingRoom,
    Joined,
    Left,
    Removed,
}
impl std::fmt::Display for ParticipantStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Registered => write!(f, "registered"),
            Self::InWaitingRoom => write!(f, "in_waiting_room"),
            Self::Joined => write!(f, "joined"),
            Self::Left => write!(f, "left"),
            Self::Removed => write!(f, "removed"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAQuestion {
    pub id: Uuid,
    pub webinar_id: Uuid,
    pub asker_id: Option<Uuid>,
    pub asker_name: String,
    pub is_anonymous: bool,
    pub question: String,
    pub status: QuestionStatus,
    pub upvotes: i32,
    pub upvoted_by: Vec<Uuid>,
    pub answer: Option<String>,
    pub answered_by: Option<Uuid>,
    pub answered_at: Option<DateTime<Utc>>,
    pub is_pinned: bool,
    pub is_highlighted: bool,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestionStatus {
    Pending,
    Approved,
    Answered,
    Dismissed,
    AnsweredLive,
}
impl std::fmt::Display for QuestionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Answered => write!(f, "answered"),
            Self::Dismissed => write!(f, "dismissed"),
            Self::AnsweredLive => write!(f, "answered_live"),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarPoll {
    pub id: Uuid,
    pub webinar_id: Uuid,
    pub question: String,
    pub poll_type: PollType,
    pub options: Vec<PollOption>,
    pub status: PollStatus,
    pub show_results_to_attendees: bool,
    pub allow_multiple_answers: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub launched_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PollType {
    SingleChoice,
    MultipleChoice,
    Rating,
    OpenEnded,
}
impl std::fmt::Display for RecordingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecordingStatus::Recording => write!(f, "recording"),
            RecordingStatus::Processing => write!(f, "processing"),
            RecordingStatus::Ready => write!(f, "ready"),
            RecordingStatus::Failed => write!(f, "failed"),
            RecordingStatus::Deleted => write!(f, "deleted"),
            RecordingStatus::Expired => write!(f, "expired"),
        }
    }
}
impl std::fmt::Display for TranscriptionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TranscriptionStatus::Pending => write!(f, "pending"),
            TranscriptionStatus::InProgress => write!(f, "in_progress"),
            TranscriptionStatus::Completed => write!(f, "completed"),
            TranscriptionStatus::Failed => write!(f, "failed"),
            TranscriptionStatus::PartiallyCompleted => write!(f, "partially_completed"),
        }
    }
}
