use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PollStatus {
    Draft,
    Launched,
    Closed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollOption {
    pub id: Uuid,
    pub text: String,
    pub vote_count: i32,
    pub percentage: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PollVote {
    pub poll_id: Uuid,
    pub participant_id: Uuid,
    pub option_ids: Vec<Uuid>,
    pub open_response: Option<String>,
    pub voted_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarRegistration {
    pub id: Uuid,
    pub webinar_id: Uuid,
    pub email: String,
    pub name: String,
    pub custom_fields: HashMap<String, String>,
    pub status: RegistrationStatus,
    pub join_link: String,
    pub registered_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub cancelled_at: Option<DateTime<Utc>>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationStatus {
    Pending,
    Confirmed,
    Cancelled,
    Attended,
    NoShow,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarAnalytics {
    pub webinar_id: Uuid,
    pub total_registrations: u32,
    pub total_attendees: u32,
    pub peak_attendees: u32,
    pub average_watch_time_seconds: u64,
    pub total_questions: u32,
    pub answered_questions: u32,
    pub total_reactions: u32,
    pub poll_participation_rate: f32,
    pub engagement_score: f32,
    pub attendee_retention: Vec<RetentionPoint>,
    /// Recording information if available
    pub recording: Option<WebinarRecording>,
    /// Transcription information if available
    pub transcription: Option<WebinarTranscription>,
}
/// Webinar recording information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarRecording {
    pub id: Uuid,
    pub webinar_id: Uuid,
    pub status: RecordingStatus,
    pub duration_seconds: u64,
    pub file_size_bytes: u64,
    pub file_url: Option<String>,
    pub download_url: Option<String>,
    pub quality: RecordingQuality,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub processed_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub view_count: u32,
    pub download_count: u32,
}
/// Recording status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecordingStatus {
    Recording,
    Processing,
    Ready,
    Failed,
    Deleted,
    Expired,
}
/// Webinar transcription information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarTranscription {
    pub id: Uuid,
    pub webinar_id: Uuid,
    pub recording_id: Uuid,
    pub status: TranscriptionStatus,
    pub language: String,
    pub duration_seconds: u64,
    pub word_count: u32,
    pub speaker_count: u32,
    pub segments: Vec<TranscriptionSegment>,
    pub full_text: Option<String>,
    pub vtt_url: Option<String>,
    pub srt_url: Option<String>,
    pub json_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub confidence_score: f32,
}
/// Transcription status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TranscriptionStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    PartiallyCompleted,
}
/// A segment of transcription with timing and speaker info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub id: Uuid,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub text: String,
    pub speaker_id: Option<String>,
    pub speaker_name: Option<String>,
    pub confidence: f32,
    pub words: Vec<TranscriptionWord>,
}
/// Individual word in transcription with timing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionWord {
    pub word: String,
    pub start_time_ms: u64,
    pub end_time_ms: u64,
    pub confidence: f32,
}
/// Request to start recording
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartRecordingRequest {
    pub quality: Option<RecordingQuality>,
    pub enable_transcription: Option<bool>,
    pub transcription_language: Option<String>,
}
/// Request to get transcription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetTranscriptionRequest {
    pub format: TranscriptionFormat,
    pub include_timestamps: bool,
    pub include_speaker_names: bool,
}
/// Transcription output format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TranscriptionFormat {
    PlainText,
    Vtt,
    Srt,
    Json,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPoint {
    pub minutes_from_start: i32,
    pub attendee_count: i32,
    pub percentage: f32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateWebinarRequest {
    pub title: String,
    pub description: Option<String>,
    pub scheduled_start: DateTime<Utc>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub settings: Option<WebinarSettings>,
    pub registration_required: bool,
    pub panelists: Option<Vec<PanelistInvite>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelistInvite {
    pub email: String,
    pub name: String,
    pub role: ParticipantRole,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateWebinarRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub scheduled_start: Option<DateTime<Utc>>,
    pub scheduled_end: Option<DateTime<Utc>>,
    pub settings: Option<WebinarSettings>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub name: String,
    pub email: String,
    pub custom_fields: Option<HashMap<String, String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitQuestionRequest {
    pub question: String,
    pub is_anonymous: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerQuestionRequest {
    pub answer: String,
    pub mark_as_live: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePollRequest {
    pub question: String,
    pub poll_type: PollType,
    pub options: Vec<String>,
    pub allow_multiple_answers: Option<bool>,
    pub show_results_to_attendees: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotePollRequest {
    pub option_ids: Vec<Uuid>,
    pub open_response: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleChangeRequest {
    pub participant_id: Uuid,
    pub new_role: ParticipantRole,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebinarEvent {
    pub event_type: WebinarEventType,
    pub webinar_id: Uuid,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebinarEventType {
    WebinarStarted,
    WebinarEnded,
    WebinarPaused,
    WebinarResumed,
    ParticipantJoined,
    ParticipantLeft,
    HandRaised,
    HandLowered,
    RoleChanged,
    QuestionSubmitted,
    QuestionAnswered,
    PollLaunched,
    PollClosed,
    ReactionSent,
    PresenterChanged,
    ScreenShareStarted,
    ScreenShareEnded,
    // Recording events
    RecordingStarted,
    RecordingStopped,
    RecordingPaused,
    RecordingResumed,
    RecordingProcessed,
    RecordingFailed,
    // Transcription events
    TranscriptionStarted,
    TranscriptionCompleted,
    TranscriptionFailed,
    TranscriptionSegmentReady,
}
