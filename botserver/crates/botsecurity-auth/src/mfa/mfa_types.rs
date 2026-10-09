use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;
use tracing::warn;

pub const TOTP_DIGITS: u32 = 6;
pub const TOTP_PERIOD: u64 = 30;
pub const TOTP_SECRET_LENGTH: usize = 20;
pub const RECOVERY_CODE_COUNT: usize = 10;
pub const RECOVERY_CODE_LENGTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MfaMethod {
    Totp,
    WebAuthn,
    EmailOtp,
    SmsOtp,
    RecoveryCode,
}

impl MfaMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Totp => "totp",
            Self::WebAuthn => "webauthn",
            Self::EmailOtp => "email_otp",
            Self::SmsOtp => "sms_otp",
            Self::RecoveryCode => "recovery_code",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Totp => "Authenticator App",
            Self::WebAuthn => "Security Key",
            Self::EmailOtp => "Email Code",
            Self::SmsOtp => "SMS Code",
            Self::RecoveryCode => "Recovery Code",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MfaStatus {
    NotEnrolled,
    Pending,
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaConfig {
    pub require_mfa: bool,
    pub allowed_methods: Vec<MfaMethod>,
    pub totp_issuer: String,
    pub totp_algorithm: TotpAlgorithm,
    pub totp_digits: u32,
    pub totp_period: u64,
    pub otp_expiry_seconds: u64,
    pub max_verification_attempts: u32,
    pub lockout_duration_minutes: u32,
    pub recovery_code_count: usize,
}

impl Default for MfaConfig {
    fn default() -> Self {
        Self {
            require_mfa: false,
            allowed_methods: vec![MfaMethod::Totp, MfaMethod::WebAuthn, MfaMethod::RecoveryCode],
            totp_issuer: "GeneralBots".into(),
            totp_algorithm: TotpAlgorithm::Sha1,
            totp_digits: TOTP_DIGITS,
            totp_period: TOTP_PERIOD,
            otp_expiry_seconds: 300,
            max_verification_attempts: 5,
            lockout_duration_minutes: 15,
            recovery_code_count: RECOVERY_CODE_COUNT,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TotpAlgorithm {
    Sha1,
    Sha256,
    Sha512,
}

impl TotpAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sha1 => "SHA1",
            Self::Sha256 => "SHA256",
            Self::Sha512 => "SHA512",
        }
    }
}

use sha2::Digest;

impl TotpAlgorithm {
    pub fn digest(data: &[u8]) -> sha2::digest::Output<Sha256> {
        Sha256::digest(data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TotpEnrollment {
    pub user_id: Uuid,
    pub secret: String,
    pub issuer: String,
    pub account_name: String,
    pub algorithm: TotpAlgorithm,
    pub digits: u32,
    pub period: u64,
    pub created_at: DateTime<Utc>,
    pub verified: bool,
}

impl TotpEnrollment {
    pub fn new(user_id: Uuid, account_name: &str, config: &MfaConfig) -> Self {
        Self {
            user_id,
            secret: super::mfa_helpers::generate_totp_secret(),
            issuer: config.totp_issuer.clone(),
            account_name: account_name.to_string(),
            algorithm: config.totp_algorithm,
            digits: config.totp_digits,
            period: config.totp_period,
            created_at: Utc::now(),
            verified: false,
        }
    }

    pub fn to_uri(&self) -> String {
        let encoded_issuer = urlencoding::encode(&self.issuer);
        let encoded_account = urlencoding::encode(&self.account_name);
        let encoded_secret = super::mfa_helpers::base32_encode(&self.secret);

        format!(
            "otpauth://totp/{encoded_issuer}:{encoded_account}?secret={encoded_secret}&issuer={encoded_issuer}&algorithm={}&digits={}&period={}",
            self.algorithm.as_str(),
            self.digits,
            self.period
        )
    }

    pub fn secret_base32(&self) -> String {
        super::mfa_helpers::base32_encode(&self.secret)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnCredential {
    pub id: String,
    pub user_id: Uuid,
    pub credential_id: Vec<u8>,
    pub public_key: Vec<u8>,
    pub counter: u32,
    pub device_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub transports: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebAuthnChallenge {
    pub challenge: Vec<u8>,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub is_registration: bool,
}

impl WebAuthnChallenge {
    pub fn new_registration(user_id: Uuid, expiry_seconds: u64) -> Self {
        let now = Utc::now();
        Self {
            challenge: super::mfa_helpers::generate_challenge(),
            user_id,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(expiry_seconds as i64),
            is_registration: true,
        }
    }

    pub fn new_authentication(user_id: Uuid, expiry_seconds: u64) -> Self {
        let now = Utc::now();
        Self {
            challenge: super::mfa_helpers::generate_challenge(),
            user_id,
            created_at: now,
            expires_at: now + chrono::Duration::seconds(expiry_seconds as i64),
            is_registration: false,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn challenge_base64(&self) -> String {
        BASE64.encode(&self.challenge)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryCode {
    pub code_hash: String,
    pub created_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
}

impl RecoveryCode {
    pub fn generate_set(count: usize) -> (Vec<String>, Vec<Self>) {
        let mut codes = Vec::with_capacity(count);
        let mut recovery_codes = Vec::with_capacity(count);
        let now = Utc::now();

        for _ in 0..count {
            let code = super::mfa_helpers::generate_recovery_code();
            let hash = super::mfa_helpers::hash_recovery_code(&code);

            codes.push(code);
            recovery_codes.push(Self {
                code_hash: hash,
                created_at: now,
                used_at: None,
            });
        }

        (codes, recovery_codes)
    }

    pub fn verify(&self, code: &str) -> bool {
        if self.used_at.is_some() {
            return false;
        }
        super::mfa_helpers::verify_recovery_code(code, &self.code_hash)
    }

    pub fn mark_used(&mut self) {
        self.used_at = Some(Utc::now());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMfaState {
    pub user_id: Uuid,
    pub status: MfaStatus,
    pub enabled_methods: Vec<MfaMethod>,
    pub totp_enrollment: Option<TotpEnrollment>,
    pub webauthn_credentials: Vec<WebAuthnCredential>,
    pub recovery_codes: Vec<RecoveryCode>,
    pub verification_attempts: u32,
    pub locked_until: Option<DateTime<Utc>>,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub preferred_method: Option<MfaMethod>,
}

impl UserMfaState {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            status: MfaStatus::NotEnrolled,
            enabled_methods: Vec::new(),
            totp_enrollment: None,
            webauthn_credentials: Vec::new(),
            recovery_codes: Vec::new(),
            verification_attempts: 0,
            locked_until: None,
            last_verified_at: None,
            preferred_method: None,
        }
    }

    pub fn is_enrolled(&self) -> bool {
        !self.enabled_methods.is_empty()
    }

    pub fn is_locked(&self) -> bool {
        if let Some(locked_until) = self.locked_until {
            Utc::now() < locked_until
        } else {
            false
        }
    }

    pub fn has_method(&self, method: MfaMethod) -> bool {
        self.enabled_methods.contains(&method)
    }

    pub fn available_recovery_codes(&self) -> usize {
        self.recovery_codes.iter().filter(|c| c.used_at.is_none()).count()
    }

    pub fn record_attempt(&mut self, success: bool, lockout_threshold: u32, lockout_minutes: u32) {
        if success {
            self.verification_attempts = 0;
            self.locked_until = None;
            self.last_verified_at = Some(Utc::now());
        } else {
            self.verification_attempts += 1;
            if self.verification_attempts >= lockout_threshold {
                self.locked_until =
                    Some(Utc::now() + chrono::Duration::minutes(lockout_minutes as i64));
                warn!(
                    "User {} locked out due to {} failed MFA attempts",
                    self.user_id, self.verification_attempts
                );
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpChallenge {
    pub id: String,
    pub user_id: Uuid,
    pub method: MfaMethod,
    pub code_hash: String,
    pub destination: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub verified: bool,
}

impl OtpChallenge {
    pub fn new(user_id: Uuid, method: MfaMethod, destination: &str, expiry_seconds: u64) -> (String, Self) {
        let code = super::mfa_helpers::generate_otp_code();
        let now = Utc::now();

        let challenge = Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            method,
            code_hash: super::mfa_helpers::hash_otp_code(&code),
            destination: super::mfa_helpers::mask_destination(destination, method),
            created_at: now,
            expires_at: now + chrono::Duration::seconds(expiry_seconds as i64),
            verified: false,
        };

        (code, challenge)
    }

    pub fn verify(&self, code: &str) -> bool {
        if self.verified || self.is_expired() {
            return false;
        }
        super::mfa_helpers::verify_otp_code(code, &self.code_hash)
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}
