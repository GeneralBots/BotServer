use super::*;

impl GuildChannel {
    pub fn is_text_channel(&self) -> bool {
        self.channel_type == 0
    }

    pub fn is_voice_channel(&self) -> bool {
        self.channel_type == 2
    }

    pub fn is_category(&self) -> bool {
        self.channel_type == 4
    }

    pub fn is_announcement_channel(&self) -> bool {
        self.channel_type == 5
    }
}
