use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub org_id: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Contact {
    pub fn new(name: String, email: String, phone: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            phone,
            org_id: String::new(),
            tags: Vec::new(),
            notes: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_org(mut self, org_id: &str) -> Self {
        self.org_id = org_id.to_string();
        self
    }

    pub fn add_tag(&mut self, tag: &str) {
        let t = tag.to_string();
        if !self.tags.contains(&t) {
            self.tags.push(t);
            self.updated_at = Utc::now();
        }
    }

    pub fn update(&mut self, name: Option<String>, email: Option<String>, phone: Option<String>) {
        if let Some(n) = name { self.name = n; }
        if let Some(e) = email { self.email = e; }
        if let Some(p) = phone { self.phone = p; }
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LeadStatus {
    New,
    Contacted,
    Qualified,
    Converted,
    Lost,
}

impl LeadStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Contacted => "contacted",
            Self::Qualified => "qualified",
            Self::Converted => "converted",
            Self::Lost => "lost",
        }
    }

    pub fn from_label(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "contacted" => Self::Contacted,
            "qualified" => Self::Qualified,
            "converted" => Self::Converted,
            "lost" => Self::Lost,
            _ => Self::New,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lead {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub source: String,
    pub status: LeadStatus,
    pub score: i32,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Lead {
    pub fn new(name: String, email: String, source: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            email,
            phone: String::new(),
            source,
            status: LeadStatus::New,
            score: 0,
            assigned_to: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn assign(&mut self, agent_id: &str) {
        self.assigned_to = Some(agent_id.to_string());
        self.updated_at = Utc::now();
    }

    pub fn advance(&mut self) {
        self.status = match self.status {
            LeadStatus::New => LeadStatus::Contacted,
            LeadStatus::Contacted => LeadStatus::Qualified,
            LeadStatus::Qualified => LeadStatus::Converted,
            LeadStatus::Converted | LeadStatus::Lost => return,
        };
        self.updated_at = Utc::now();
    }

    pub fn lose(&mut self) {
        self.status = LeadStatus::Lost;
        self.updated_at = Utc::now();
    }

    pub fn update_score(&mut self, delta: i32) {
        self.score = (self.score + delta).max(0);
        self.updated_at = Utc::now();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: Uuid,
    pub title: String,
    pub contact_id: Uuid,
    pub value: f64,
    pub stage: String,
    pub probability: f64,
    pub expected_close: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Opportunity {
    pub fn new(title: String, contact_id: Uuid, value: f64) -> Self {
        Self {
            id: Uuid::new_v4(),
            contact_id,
            title,
            value,
            stage: "prospecting".to_string(),
            probability: 10.0,
            expected_close: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}