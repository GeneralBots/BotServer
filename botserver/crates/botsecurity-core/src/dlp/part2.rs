use super::*;

impl SensitiveDataType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::PhoneNumber => "phone_number",
            Self::CreditCard => "credit_card",
            Self::Ssn => "ssn",
            Self::IpAddress => "ip_address",
            Self::ApiKey => "api_key",
            Self::Password => "password",
            Self::AwsAccessKey => "aws_access_key",
            Self::AwsSecretKey => "aws_secret_key",
            Self::PrivateKey => "private_key",
            Self::JwtToken => "jwt_token",
            Self::BankAccount => "bank_account",
            Self::PassportNumber => "passport_number",
            Self::DriversLicense => "drivers_license",
            Self::HealthId => "health_id",
            Self::Custom => "custom",
        }
    }

    pub fn severity(&self) -> DlpSeverity {
        match self {
            Self::CreditCard | Self::Ssn | Self::BankAccount | Self::PrivateKey => {
                DlpSeverity::Critical
            }
            Self::AwsAccessKey
            | Self::AwsSecretKey
            | Self::ApiKey
            | Self::Password
            | Self::JwtToken => DlpSeverity::High,
            Self::PassportNumber | Self::DriversLicense | Self::HealthId => DlpSeverity::High,
            Self::Email | Self::PhoneNumber => DlpSeverity::Medium,
            Self::IpAddress => DlpSeverity::Low,
            Self::Custom => DlpSeverity::Medium,
        }
    }

    pub fn redaction_pattern(&self) -> &'static str {
        match self {
            Self::Email => "[EMAIL REDACTED]",
            Self::PhoneNumber => "[PHONE REDACTED]",
            Self::CreditCard => "[CARD REDACTED]",
            Self::Ssn => "[SSN REDACTED]",
            Self::IpAddress => "[IP REDACTED]",
            Self::ApiKey => "[API KEY REDACTED]",
            Self::Password => "[PASSWORD REDACTED]",
            Self::AwsAccessKey => "[AWS KEY REDACTED]",
            Self::AwsSecretKey => "[AWS SECRET REDACTED]",
            Self::PrivateKey => "[PRIVATE KEY REDACTED]",
            Self::JwtToken => "[TOKEN REDACTED]",
            Self::BankAccount => "[BANK ACCOUNT REDACTED]",
            Self::PassportNumber => "[PASSPORT REDACTED]",
            Self::DriversLicense => "[LICENSE REDACTED]",
            Self::HealthId => "[HEALTH ID REDACTED]",
            Self::Custom => "[REDACTED]",
        }
    }
}
