use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketStatus {
    Open,
    Pending,
    Resolved,
    Closed,
}

impl TicketStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Pending => "pending",
            Self::Resolved => "resolved",
            Self::Closed => "closed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pending" => Self::Pending,
            "resolved" => Self::Resolved,
            "closed" => Self::Closed,
            _ => Self::Open,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TicketPriority {
    Low,
    Medium,
    High,
    Urgent,
}

impl TicketPriority {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Urgent => "urgent",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "low" => Self::Low,
            "high" => Self::High,
            "urgent" => Self::Urgent,
            _ => Self::Medium,
        }
    }

    pub fn sla_hours(&self) -> i64 {
        match self {
            Self::Low => 72,
            Self::Medium => 24,
            Self::High => 8,
            Self::Urgent => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: Uuid,
    pub ticket_number: String,
    pub subject: String,
    pub description: String,
    pub status: TicketStatus,
    pub priority: TicketPriority,
    pub assigned_to: Option<String>,
    pub created_by: String,
    pub channel: String,
    pub sla_deadline: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl Ticket {
    pub fn new(subject: &str, description: &str, created_by: &str) -> Self {
        let now = Utc::now();
        let ticket_number = format!(
            "TKT-{}-{}",
            now.format("%Y%m"),
            Uuid::new_v4().to_string().split('-').next().unwrap_or("0000").to_uppercase()
        );

        Self {
            id: Uuid::new_v4(),
            ticket_number,
            subject: subject.to_string(),
            description: description.to_string(),
            status: TicketStatus::Open,
            priority: TicketPriority::Medium,
            assigned_to: None,
            created_by: created_by.to_string(),
            channel: "web".to_string(),
            sla_deadline: None,
            created_at: now,
            updated_at: now,
            closed_at: None,
        }
    }

    pub fn assign(&mut self, agent_id: &str) {
        self.assigned_to = Some(agent_id.to_string());
        self.updated_at = Utc::now();
    }

    pub fn resolve(&mut self) {
        self.status = TicketStatus::Resolved;
        self.updated_at = Utc::now();
    }

    pub fn close(&mut self) {
        self.status = TicketStatus::Closed;
        self.closed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    pub fn reopen(&mut self) {
        self.status = TicketStatus::Open;
        self.closed_at = None;
        self.updated_at = Utc::now();
    }

    pub fn set_priority(&mut self, priority: TicketPriority) {
        self.priority = priority;
        self.sla_deadline = self.calculate_sla();
        self.updated_at = Utc::now();
    }

    pub fn calculate_sla(&self) -> Option<DateTime<Utc>> {
        Some(Utc::now() + Duration::hours(self.priority.sla_hours()))
    }

    pub fn is_sla_breached(&self) -> bool {
        self.sla_deadline.is_some_and(|d| Utc::now() > d)
    }

    pub fn time_to_sla_breach(&self) -> Option<Duration> {
        self.sla_deadline.map(|d| d - Utc::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ticket_new() {
        let t = Ticket::new("Bug", "Something broke", "user@ex.com");
        assert!(t.ticket_number.starts_with("TKT-"));
        assert_eq!(t.status, TicketStatus::Open);
        assert_eq!(t.priority, TicketPriority::Medium);
    }

    #[test]
    fn test_ticket_lifecycle() {
        let mut t = Ticket::new("Issue", "Desc", "user");
        assert_eq!(t.status, TicketStatus::Open);
        t.resolve();
        assert_eq!(t.status, TicketStatus::Resolved);
        t.close();
        assert_eq!(t.status, TicketStatus::Closed);
        assert!(t.closed_at.is_some());
        t.reopen();
        assert_eq!(t.status, TicketStatus::Open);
        assert!(t.closed_at.is_none());
    }

    #[test]
    fn test_sla_calculation() {
        let mut t = Ticket::new("Urgent", "Critical", "user");
        t.set_priority(TicketPriority::Urgent);
        assert!(t.sla_deadline.is_some());
        assert!(!t.is_sla_breached());
    }

    #[test]
    fn test_ticket_number_format() {
        let t = Ticket::new("Test", "Test", "user");
        assert!(t.ticket_number.len() > 10);
    }

    #[test]
    fn test_assign() {
        let mut t = Ticket::new("Test", "Test", "user");
        t.assign("agent007");
        assert_eq!(t.assigned_to, Some("agent007".to_string()));
    }
}
