use super::*;

impl Default for MeetingSettings {
    fn default() -> Self {
        Self {
            enable_transcription: true,
            enable_recording: false,
            enable_chat: true,
            enable_screen_share: true,
            auto_admit: true,
            waiting_room: false,
            bot_enabled: true,
            bot_id: None,
        }
    }
}
