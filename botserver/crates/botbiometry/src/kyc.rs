//! KYC (Know Your Customer) state machine and document types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Lifecycle states for a KYC case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KycState {
    /// Awaiting document submission.
    Pending,
    /// Some documents submitted, others still required.
    Incomplete,
    /// All documents submitted, awaiting automated + manual review.
    UnderReview,
    /// Liveness challenge issued to the applicant.
    LivenessRequired,
    /// Approved by the system; awaiting Zitadel identity creation.
    Approved,
    /// Rejected; reason required.
    Rejected,
    /// Expired (periodic re-verification due).
    Expired,
}

impl KycState {
    /// Returns true if the state is terminal (Approved, Rejected or Expired).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Approved | Self::Rejected | Self::Expired)
    }

    /// Wire status of this state, as used by the KYC APIs.
    pub fn as_status(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Incomplete => "incomplete",
            Self::UnderReview => "under_review",
            Self::LivenessRequired => "liveness_required",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }

    /// Parses a wire status; `None` when the caller sent an unknown state.
    pub fn from_status(status: &str) -> Option<Self> {
        match status {
            "pending" => Some(Self::Pending),
            "incomplete" => Some(Self::Incomplete),
            "under_review" | "in_review" | "review" => Some(Self::UnderReview),
            "liveness_required" => Some(Self::LivenessRequired),
            "approved" => Some(Self::Approved),
            "rejected" => Some(Self::Rejected),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

/// Kind of identity document uploaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentKind {
    /// National ID card (RG in Brazil).
    NationalId,
    /// Driver's license.
    DriversLicense,
    /// Passport.
    Passport,
    /// Taxpayer registry (CPF/CNPJ in Brazil).
    TaxId,
    /// Proof of address (utility bill, bank statement).
    ProofOfAddress,
    /// Selfie holding the document.
    SelfieWithDocument,
    /// Other (free-text description required).
    Other,
}

/// A single KYC document uploaded by the applicant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KycDocument {
    /// Server-assigned document ID.
    pub id: Uuid,
    /// Document kind.
    pub kind: DocumentKind,
    /// Original filename as uploaded.
    pub filename: String,
    /// MIME type (`image/jpeg`, `application/pdf`).
    pub mime: String,
    /// SHA-256 of the bytes (hex).
    pub sha256: String,
    /// Storage path in Drive (`{tenant}/kyc/{case_id}/{doc_id}`).
    pub drive_path: String,
    /// Upload timestamp.
    pub uploaded_at: DateTime<Utc>,
    /// Whether an automated OCR/fraud check passed.
    pub auto_validated: bool,
}

/// Full KYC case for a single applicant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KycCase {
    /// Server-assigned case ID.
    pub id: Uuid,
    /// Tenant (Zitadel org) owning this case.
    pub tenant_id: String,
    /// Optional Zitadel user ID once provisioned.
    pub zitadel_user_id: Option<String>,
    /// Current state.
    pub state: KycState,
    /// Reason for rejection (if [`KycState::Rejected`]).
    pub rejection_reason: Option<String>,
    /// All documents uploaded so far.
    pub documents: Vec<KycDocument>,
    /// Case creation time.
    pub created_at: DateTime<Utc>,
    /// Last state change.
    pub updated_at: DateTime<Utc>,
}

impl KycCase {
    /// Create a new pending case for a tenant.
    pub fn new(tenant_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            tenant_id: tenant_id.into(),
            zitadel_user_id: None,
            state: KycState::Pending,
            rejection_reason: None,
            documents: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a document and move to `Incomplete` if previously `Pending`.
    pub fn add_document(&mut self, doc: KycDocument) {
        if self.state == KycState::Pending {
            self.state = KycState::Incomplete;
        }
        self.documents.push(doc);
        self.updated_at = Utc::now();
    }

    /// Mark the case as ready for review.
    pub fn submit_for_review(&mut self) {
        if self.state == KycState::Incomplete {
            self.state = KycState::UnderReview;
            self.updated_at = Utc::now();
        }
    }
}

/// Errors that may surface during KYC validation.
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
pub enum KycError {
    /// Document type rejected for this case.
    #[error("document kind {0:?} is not acceptable for this KYC level")]
    UnacceptableDocument(DocumentKind),
    /// State transition not allowed.
    #[error("illegal state transition from {from:?} to {to:?}")]
    IllegalTransition {
        /// Current state.
        from: KycState,
        /// Target state.
        to: KycState,
    },
    /// Zitadel identity not yet created.
    #[error("zitadel user not provisioned yet")]
    NotProvisioned,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_case_is_pending() {
        let case = KycCase::new("tenant-1");
        assert_eq!(case.state, KycState::Pending);
        assert!(case.documents.is_empty());
    }

    #[test]
    fn add_document_moves_to_incomplete() {
        let mut case = KycCase::new("t1");
        let doc = KycDocument {
            id: Uuid::new_v4(),
            kind: DocumentKind::Passport,
            filename: "passport.jpg".to_string(),
            mime: "image/jpeg".to_string(),
            sha256: "0".repeat(64),
            drive_path: "t1/kyc/x/y".to_string(),
            uploaded_at: Utc::now(),
            auto_validated: true,
        };
        case.add_document(doc);
        assert_eq!(case.state, KycState::Incomplete);
        assert_eq!(case.documents.len(), 1);
    }

    #[test]
    fn is_terminal_only_for_terminal_states() {
        assert!(!KycState::Pending.is_terminal());
        assert!(KycState::Approved.is_terminal());
        assert!(KycState::Rejected.is_terminal());
        assert!(KycState::Expired.is_terminal());
    }

    #[test]
    fn wire_statuses_round_trip() {
        for state in [
            KycState::Pending,
            KycState::Incomplete,
            KycState::UnderReview,
            KycState::LivenessRequired,
            KycState::Approved,
            KycState::Rejected,
            KycState::Expired,
        ] {
            assert_eq!(KycState::from_status(state.as_status()), Some(state));
        }
        assert_eq!(KycState::from_status("in_review"), Some(KycState::UnderReview));
        assert_eq!(KycState::from_status("bogus"), None);
    }
}
