use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;
use log::info;
use sha2::{Sha512, Digest};

use crate::minutes::types::{MinuteSignature, MinuteStatus, MeetingMinute};

#[derive(Debug, Clone)]
pub struct DigitalSigner;

impl DigitalSigner {
    pub fn new() -> Self {
        Self
    }

    pub fn sign(&self, minute: &MeetingMinute) -> Result<MinuteSignature> {
        let content = format!("{:?}", minute);
        let mut hasher = Sha512::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());
        Ok(MinuteSignature {
            id: Uuid::new_v4(),
            minute_id: minute.id,
            user_id: Uuid::nil(),
            signature_id: None,
            signed_hash: hash,
            signed_at: Utc::now(),
            ip_address: None,
        })
    }
}

pub struct SignatureService;

impl SignatureService {
    pub fn sign_minute(minute: &MeetingMinute, user_id: Uuid, signature_id: Option<Uuid>, ip_address: Option<String>) -> MinuteSignature {
        let content = Self::canonical_content(minute);
        let mut hasher = Sha512::new();
        hasher.update(content.as_bytes());
        let hash = hex::encode(hasher.finalize());

        info!("Minute {} signed by user {} with hash {}", minute.id, user_id, &hash[..16]);

        MinuteSignature {
            id: Uuid::new_v4(),
            minute_id: minute.id,
            user_id,
            signature_id,
            signed_hash: hash,
            signed_at: Utc::now(),
            ip_address,
        }
    }

    pub fn verify_signature(signature: &MinuteSignature, minute: &MeetingMinute) -> bool {
        let content = Self::canonical_content(minute);
        let mut hasher = Sha512::new();
        hasher.update(content.as_bytes());
        let expected_hash = hex::encode(hasher.finalize());
        expected_hash == signature.signed_hash
    }

    pub fn is_signed_by_user(signatures: &[MinuteSignature], user_id: Uuid) -> bool {
        signatures.iter().any(|s| s.user_id == user_id)
    }

    pub fn min_signatures_required() -> usize { 1 }

    pub fn can_finalize(signatures: &[MinuteSignature], total_attendees: usize) -> bool {
        if total_attendees == 0 {
            return !signatures.is_empty();
        }
        let min_required = (total_attendees as f64 * 0.5).ceil() as usize;
        signatures.len() >= min_required.max(Self::min_signatures_required())
    }

    fn canonical_content(minute: &MeetingMinute) -> String {
        let mut content = String::new();
        content.push_str(&format!("id:{}\n", minute.id));
        content.push_str(&format!("title:{}\n", minute.title));
        content.push_str(&format!("summary:{}\n", minute.summary));
        for kp in &minute.key_points {
            content.push_str(&format!("key_point:{kp}\n"));
        }
        for ai in &minute.action_items {
            content.push_str(&format!("action:{}|{}|{:?}\n", ai.task, ai.assignee.as_deref().unwrap_or(""), ai.due_date.as_deref().unwrap_or("")));
        }
        for d in &minute.decisions {
            content.push_str(&format!("decision:{d}\n"));
        }
        for a in &minute.attendees {
            content.push_str(&format!("attendee:{}|{}\n", a.name, a.role.as_deref().unwrap_or("")));
        }
        content.push_str(&format!("version:{}\n", minute.version));
        content
    }

    pub fn update_minute_status(minute: &mut MeetingMinute, signatures: &[MinuteSignature]) {
        if signatures.is_empty() {
            minute.status = MinuteStatus::Draft;
        } else if Self::can_finalize(signatures, minute.attendees.len()) {
            minute.status = MinuteStatus::Signed;
        } else {
            minute.status = MinuteStatus::Final;
        }
    }
}
