#[cfg(test)]
use super::super::types::{WebinarStatus, ParticipantRole};

#[test]
fn test_webinar_status_display() {
    assert_eq!(WebinarStatus::Draft.to_string(), "draft");
    assert_eq!(WebinarStatus::Live.to_string(), "live");
    assert_eq!(WebinarStatus::Ended.to_string(), "ended");
}

#[test]
fn test_participant_role_can_present() {
    assert!(ParticipantRole::Host.can_present());
    assert!(ParticipantRole::Presenter.can_present());
    assert!(!ParticipantRole::Attendee.can_present());
}
