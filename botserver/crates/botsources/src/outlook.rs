use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::sharepoint::SharePointClient;
use super::m365_auth::M365Token;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailMessage {
    pub id: String,
    pub conversation_id: Option<String>,
    pub subject: String,
    pub body_preview: String,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub from: EmailRecipient,
    pub to: Vec<EmailRecipient>,
    pub cc: Vec<EmailRecipient>,
    pub bcc: Vec<EmailRecipient>,
    pub received_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub is_read: bool,
    pub has_attachments: bool,
    pub importance: EmailImportance,
    pub categories: Vec<String>,
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EmailImportance {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailRecipient {
    pub name: Option<String>,
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: String,
    pub subject: String,
    pub body_preview: Option<String>,
    pub body_html: Option<String>,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub is_all_day: bool,
    pub location: Option<String>,
    pub organizer: Option<EmailRecipient>,
    pub attendees: Vec<EmailRecipient>,
    pub response_status: ResponseStatus,
    pub is_online_meeting: bool,
    pub join_url: Option<String>,
    pub categories: Vec<String>,
    pub show_as: ShowAs,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResponseStatus {
    None,
    Organizer,
    TentativelyAccepted,
    Accepted,
    Declined,
    NotResponded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShowAs {
    Free,
    Tentative,
    Busy,
    Oof,
    WorkingElsewhere,
    Unknown,
}

pub struct OutlookService {
    pub client: SharePointClient,
}

impl OutlookService {
    pub fn new(client: SharePointClient) -> Self {
        Self { client }
    }

    pub fn me_messages_url(&self, top: u32) -> Result<String, String> {
        let url = format!("{}/me/messages?$top={}", self.client.graph_base_url, top);
        SharePointClient::validate_url_for_graph(&url)?;
        Ok(url)
    }

    pub fn me_calendar_view_url(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<String, String> {
        let url = format!(
            "{}/me/calendarView?startDateTime={}&endDateTime={}",
            self.client.graph_base_url,
            start.to_rfc3339(),
            end.to_rfc3339()
        );
        SharePointClient::validate_url_for_graph(&url)?;
        Ok(url)
    }

    pub fn send_mail_url(&self) -> String {
        format!("{}/me/sendMail", self.client.graph_base_url)
    }

    pub fn build_auth_header(token: &M365Token) -> String {
        format!("Bearer {}", token.access_token)
    }

    pub fn is_working_hours(event: &CalendarEvent) -> bool {
        let hour = event.start.format("%H").to_string();
        let h: u32 = hour.parse().unwrap_or(0);
        (8..18).contains(&h)
    }

    pub fn count_meetings_in_range(events: &[CalendarEvent], date: NaiveDate) -> u32 {
        events
            .iter()
            .filter(|e| e.start.date_naive() == date)
            .count() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_urls_have_required_segments() {
        let client = SharePointClient::new("tid".into(), "tok".into());
        let svc = OutlookService::new(client);
        assert!(svc.me_messages_url(10).expect("graph URL").contains("/me/messages"));
        assert!(svc.send_mail_url().contains("/me/sendMail"));
    }
}
