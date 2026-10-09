pub(crate) use regex::Regex;
pub(crate) use std::sync::LazyLock;
pub(crate) use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    Required(String),
    TooShort { field: String, min: usize, actual: usize },
    TooLong { field: String, max: usize, actual: usize },
    InvalidFormat { field: String, expected: String },
    InvalidRange { field: String, min: String, max: String },
    InvalidValue { field: String, message: String },
    InvalidEmail(String),
    InvalidUrl(String),
    InvalidUuid(String),
    InvalidPhone(String),
    Forbidden { field: String, reason: String },
    Custom(String),
}

#[derive(Debug, Default)]
pub struct ValidationResult {
    errors: Vec<ValidationError>,
}

static EMAIL_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$"
    ).ok());

static URL_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(
        r"^https?://[a-zA-Z0-9][-a-zA-Z0-9]*(\.[a-zA-Z0-9][-a-zA-Z0-9]*)+(/[-a-zA-Z0-9()@:%_\+.~#?&/=]*)?$"
    ).ok());

static UUID_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(
        r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$"
    ).ok());

static PHONE_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"^\+?[1-9]\d{6,14}$").ok());

static ALPHANUMERIC_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9]+$").ok());

static SLUG_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").ok());

static USERNAME_REGEX: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{2,31}$").ok());

pub struct Validator {
    result: ValidationResult,
}

static SSRF_BLOCKED_HOSTS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "localhost",
        "127.0.0.1",
        "0.0.0.0",
        "::1",
        "[::1]",
        "169.254.169.254",
        "metadata.google.internal",
        "instance-data",
        "linklocal.amazonaws.com",
        "169.254.169.254",
        "10.0.0.0",
        "10.255.255.255",
        "172.16.0.0",
        "172.31.255.255",
        "192.168.0.0",
        "192.168.255.255",
        "fc00:",
        "fd00:",
        "fe80:",
    ]
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_required() {
        assert!(validate_required(Some("value"), "field").is_ok());
        assert!(validate_required(Some(""), "field").is_err());
        assert!(validate_required(None, "field").is_err());
        assert!(validate_required(Some("  "), "field").is_err());
    }

    #[test]
    fn test_validate_length() {
        assert!(validate_length("hello", "field", Some(1), Some(10)).is_ok());
        assert!(validate_length("hi", "field", Some(3), None).is_err());
        assert!(validate_length("hello world", "field", None, Some(5)).is_err());
    }

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("user.name+tag@domain.co.uk").is_ok());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("@domain.com").is_err());
        assert!(validate_email("user@").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com").is_ok());
        assert!(validate_url("http://sub.domain.com/path?query=1").is_ok());
        assert!(validate_url("ftp://invalid.com").is_err());
        assert!(validate_url("not-a-url").is_err());
    }

    #[test]
    fn test_validate_uuid() {
        assert!(validate_uuid("550e8400-e29b-41d4-a716-446655440000").is_ok());
        assert!(validate_uuid("not-a-uuid").is_err());
        assert!(validate_uuid("550e8400-e29b-41d4-a716").is_err());
    }

    #[test]
    fn test_validate_phone() {
        assert!(validate_phone("+1234567890").is_ok());
        assert!(validate_phone("1234567890").is_ok());
        assert!(validate_phone("123").is_err());
    }

    #[test]
    fn test_validate_username() {
        assert!(validate_username("john_doe").is_ok());
        assert!(validate_username("user123").is_ok());
        assert!(validate_username("ab").is_err());
        assert!(validate_username("123user").is_err());
    }

    #[test]
    fn test_validate_password_strength() {
        assert!(validate_password_strength("SecurePass1!").is_ok());
        assert!(validate_password_strength("Weak1!").is_err());
        assert!(validate_password_strength("alllowercase").is_err());
        assert!(validate_password_strength("ALLUPPERCASE").is_err());
    }

    #[test]
    fn test_sanitize_html() {
        assert_eq!(sanitize_html("<script>alert('xss')</script>"), "&lt;script&gt;alert(&#x27;xss&#x27;)&lt;/script&gt;");
        assert_eq!(sanitize_html("Hello & World"), "Hello &amp; World");
    }

    #[test]
    fn test_strip_html_tags() {
        assert_eq!(strip_html_tags("<p>Hello</p>"), "Hello");
        assert_eq!(strip_html_tags("<script>bad</script>text"), "badtext");
    }

    #[test]
    fn test_validate_no_script_injection() {
        assert!(validate_no_script_injection("normal text", "field").is_ok());
        assert!(validate_no_script_injection("javascript:alert(1)", "field").is_err());
        assert!(validate_no_script_injection("<script>bad</script>", "field").is_err());
        assert!(validate_no_script_injection("onclick=hack", "field").is_err());
    }

    #[test]
    fn test_validator_chain() {
        let result = Validator::new()
            .string_required("test", "name")
            .length("test", "name", Some(1), Some(100))
            .no_html("test", "name")
            .validate();

        assert!(result.is_ok());
    }

    #[test]
    fn test_validator_with_errors() {
        let result = Validator::new()
            .string_required("", "name")
            .email("invalid-email")
            .validate();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.errors().len(), 2);
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::Required("username".to_string());
        assert!(err.to_string().contains("username"));
        assert!(err.to_string().contains("required"));
    }

    #[test]
    fn test_validate_slug() {
        assert!(validate_slug("my-slug", "field").is_ok());
        assert!(validate_slug("slug123", "field").is_ok());
        assert!(validate_slug("My-Slug", "field").is_err());
        assert!(validate_slug("slug_bad", "field").is_err());
    }

    #[test]
    fn test_validate_range() {
        assert!(validate_range(5, "count", Some(1), Some(10)).is_ok());
        assert!(validate_range(0, "count", Some(1), None).is_err());
        assert!(validate_range(100, "count", None, Some(50)).is_err());
    }

    #[test]
    fn test_validate_one_of() {
        assert!(validate_one_of(&"admin", "role", &["admin", "user", "guest"]).is_ok());
        assert!(validate_one_of(&"hacker", "role", &["admin", "user", "guest"]).is_err());
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part3::*;
pub use part5::*;
