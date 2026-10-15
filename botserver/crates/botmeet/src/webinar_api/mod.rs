mod constants;
mod error;
mod handlers;
pub mod migrations;
mod service;
#[cfg(test)]
mod tests;
pub mod types;

// Re-export all public types for backward compatibility
pub use constants::{MAX_RAISED_HANDS_VISIBLE, MAX_WEBINAR_PARTICIPANTS, QA_QUESTION_MAX_LENGTH};
pub use error::WebinarError;
pub use handlers::webinar_routes;
pub use migrations::create_webinar_tables_migration;
pub use service::WebinarService;
pub use types::{
    AnswerQuestionRequest, CreatePollRequest, CreateWebinarRequest, FieldType,
    GetTranscriptionRequest, PanelistInvite, PollOption, PollStatus, PollType, PollVote,
    QAQuestion, QuestionStatus, RecordingQuality, RecordingStatus, RegisterRequest,
    RegistrationField, RegistrationStatus, RetentionPoint, RoleChangeRequest,
    StartRecordingRequest, SubmitQuestionRequest, TranscriptionFormat,
    TranscriptionSegment, TranscriptionStatus, TranscriptionWord, Webinar,
    WebinarAnalytics, WebinarEvent, WebinarEventType, WebinarParticipant,
    WebinarPoll, WebinarRecording, WebinarRegistration, WebinarSettings,
    WebinarStatus, WebinarTranscription, ParticipantRole, ParticipantStatus,
};
