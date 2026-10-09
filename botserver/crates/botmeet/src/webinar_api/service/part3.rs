use super::*;

impl WebinarService {
    pub(super) fn row_to_webinar(&self, row: WebinarRow) -> Webinar {
        let settings: WebinarSettings = serde_json::from_str(&row.settings_json).unwrap_or_default();
        let status = match row.status.as_str() {
            "draft" => WebinarStatus::Draft,
            "scheduled" => WebinarStatus::Scheduled,
            "live" => WebinarStatus::Live,
            "paused" => WebinarStatus::Paused,
            "ended" => WebinarStatus::Ended,
            "cancelled" => WebinarStatus::Cancelled,
            _ => WebinarStatus::Draft,
        };

        Webinar {
            id: row.id,
            organization_id: row.organization_id,
            meeting_id: row.meeting_id,
            title: row.title,
            description: row.description,
            scheduled_start: row.scheduled_start,
            scheduled_end: row.scheduled_end,
            actual_start: row.actual_start,
            actual_end: row.actual_end,
            status,
            settings,
            registration_required: row.registration_required,
            registration_url: row.registration_url,
            host_id: row.host_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }

    pub(super) fn row_to_participant(&self, row: ParticipantRow) -> WebinarParticipant {
        let role = match row.role.as_str() {
            "host" => ParticipantRole::Host,
            "co_host" => ParticipantRole::CoHost,
            "presenter" => ParticipantRole::Presenter,
            "panelist" => ParticipantRole::Panelist,
            _ => ParticipantRole::Attendee,
        };
        let status = match row.status.as_str() {
            "registered" => ParticipantStatus::Registered,
            "in_waiting_room" => ParticipantStatus::InWaitingRoom,
            "joined" => ParticipantStatus::Joined,
            "left" => ParticipantStatus::Left,
            "removed" => ParticipantStatus::Removed,
            _ => ParticipantStatus::Registered,
        };
        let registration_data: Option<HashMap<String, String>> = row
            .registration_data
            .and_then(|d| serde_json::from_str(&d).ok());

        WebinarParticipant {
            id: row.id,
            webinar_id: row.webinar_id,
            user_id: row.user_id,
            name: row.name,
            email: row.email,
            role,
            status,
            hand_raised: row.hand_raised,
            hand_raised_at: row.hand_raised_at,
            is_speaking: row.is_speaking,
            video_enabled: row.video_enabled,
            audio_enabled: row.audio_enabled,
            screen_sharing: row.screen_sharing,
            joined_at: row.joined_at,
            left_at: row.left_at,
            registration_data,
        }
    }

    pub(super) fn row_to_question(&self, row: QuestionRow) -> QAQuestion {
        let status = match row.status.as_str() {
            "pending" => QuestionStatus::Pending,
            "approved" => QuestionStatus::Approved,
            "answered" => QuestionStatus::Answered,
            "dismissed" => QuestionStatus::Dismissed,
            "answered_live" => QuestionStatus::AnsweredLive,
            _ => QuestionStatus::Pending,
        };
        let upvoted_by: Vec<Uuid> = row
            .upvoted_by
            .and_then(|u| serde_json::from_str(&u).ok())
            .unwrap_or_default();

        QAQuestion {
            id: row.id,
            webinar_id: row.webinar_id,
            asker_id: row.asker_id,
            asker_name: row.asker_name,
            is_anonymous: row.is_anonymous,
            question: row.question,
            status,
            upvotes: row.upvotes,
            upvoted_by,
            answer: row.answer,
            answered_by: row.answered_by,
            answered_at: row.answered_at,
            is_pinned: row.is_pinned,
            is_highlighted: row.is_highlighted,
            created_at: row.created_at,
        }
    }
}
