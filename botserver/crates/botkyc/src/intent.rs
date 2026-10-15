//! Canonical request payloads for the KYC/identity BASIC keywords.
//!
//! The `VERIFY FACE`, `VALIDATE DOCUMENT`, `CAPTURE SIGNATURE` and
//! `START KYC` keywords only bind arguments; the shape of the request they
//! produce belongs to this crate (#1506), so a new consumer of the KYC
//! surface cannot invent a second spelling.

use serde::{Deserialize, Serialize};

/// What a KYC request is about. Serializes to the `kind` field the runtime
/// routes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationKind {
    FaceVerification,
    DocumentValidation,
    SignatureCapture,
    KycWorkflow,
}

impl VerificationKind {
    /// `kind` value as the runtime expects it.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::FaceVerification => "face_verification",
            Self::DocumentValidation => "document_validation",
            Self::SignatureCapture => "signature_capture",
            Self::KycWorkflow => "kyc_workflow",
        }
    }
}

/// A KYC request ready to be dispatched.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycRequest {
    pub kind: VerificationKind,
    pub action: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selfie_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_step: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_status: Option<String>,
}

impl KycRequest {
    /// `VERIFY FACE <profile>, <selfie>`
    pub fn face(profile_id: impl Into<String>, selfie_url: impl Into<String>) -> Self {
        Self {
            kind: VerificationKind::FaceVerification,
            action: "verify",
            profile_id: Some(profile_id.into()),
            selfie_url: Some(selfie_url.into()),
            status: Some("pending".to_string()),
            document_url: None,
            document_id: None,
            signature_data: None,
            workflow_name: None,
            current_step: None,
            verification_status: None,
        }
    }

    /// `VALIDATE DOCUMENT <profile>, <document>`
    pub fn document(profile_id: impl Into<String>, document_url: impl Into<String>) -> Self {
        Self {
            kind: VerificationKind::DocumentValidation,
            action: "validate",
            profile_id: Some(profile_id.into()),
            document_url: Some(document_url.into()),
            verification_status: Some("pending".to_string()),
            selfie_url: None,
            document_id: None,
            signature_data: None,
            workflow_name: None,
            status: None,
            current_step: None,
        }
    }

    /// `CAPTURE SIGNATURE <profile>, <document>, <signature>`
    pub fn signature(
        profile_id: impl Into<String>,
        document_id: impl Into<String>,
        signature_data: impl Into<String>,
    ) -> Self {
        Self {
            kind: VerificationKind::SignatureCapture,
            action: "create",
            profile_id: Some(profile_id.into()),
            document_id: Some(document_id.into()),
            signature_data: Some(signature_data.into()),
            selfie_url: None,
            document_url: None,
            workflow_name: None,
            status: None,
            current_step: None,
            verification_status: None,
        }
    }

    /// `START KYC <profile>, <workflow>`
    pub fn workflow(profile_id: impl Into<String>, workflow_name: impl Into<String>) -> Self {
        Self {
            kind: VerificationKind::KycWorkflow,
            action: "start",
            profile_id: Some(profile_id.into()),
            workflow_name: Some(workflow_name.into()),
            current_step: Some("document_upload".to_string()),
            status: Some("in_progress".to_string()),
            selfie_url: None,
            document_url: None,
            document_id: None,
            signature_data: None,
            verification_status: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_request_round_trips_to_the_runtime_shape() {
        let json = serde_json::to_value(KycRequest::face("p-1", "https://img/selfie.jpg")).unwrap();
        assert_eq!(json["kind"], "face_verification");
        assert_eq!(json["action"], "verify");
        assert_eq!(json["profile_id"], "p-1");
        assert_eq!(json["selfie_url"], "https://img/selfie.jpg");
        assert_eq!(json["status"], "pending");
        assert!(json.get("document_url").is_none());
    }

    #[test]
    fn document_request_uses_verification_status() {
        let json = serde_json::to_value(KycRequest::document("p-2", "passport.jpg")).unwrap();
        assert_eq!(json["kind"], "document_validation");
        assert_eq!(json["verification_status"], "pending");
    }

    #[test]
    fn workflow_request_starts_at_document_upload() {
        let json = serde_json::to_value(KycRequest::workflow("p-3", "onboarding")).unwrap();
        assert_eq!(json["current_step"], "document_upload");
        assert_eq!(json["status"], "in_progress");
    }
}
