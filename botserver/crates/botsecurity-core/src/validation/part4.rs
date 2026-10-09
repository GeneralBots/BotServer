use super::*;

impl Validator {
    pub fn new() -> Self {
        Self {
            result: ValidationResult::new(),
        }
    }

    pub fn required(mut self, value: Option<&str>, field_name: &str) -> Self {
        if let Err(e) = validate_required(value, field_name) {
            self.result.add_error(e);
        }
        self
    }

    pub fn string_required(mut self, value: &str, field_name: &str) -> Self {
        if let Err(e) = validate_string_required(value, field_name) {
            self.result.add_error(e);
        }
        self
    }

    pub fn length(mut self, value: &str, field_name: &str, min: Option<usize>, max: Option<usize>) -> Self {
        if let Err(e) = validate_length(value, field_name, min, max) {
            self.result.add_error(e);
        }
        self
    }

    pub fn email(mut self, value: &str) -> Self {
        if let Err(e) = validate_email(value) {
            self.result.add_error(e);
        }
        self
    }

    pub fn url(mut self, value: &str) -> Self {
        if let Err(e) = validate_url(value) {
            self.result.add_error(e);
        }
        self
    }

    pub fn uuid(mut self, value: &str) -> Self {
        if let Err(e) = validate_uuid(value) {
            self.result.add_error(e);
        }
        self
    }

    pub fn phone(mut self, value: &str) -> Self {
        if let Err(e) = validate_phone(value) {
            self.result.add_error(e);
        }
        self
    }

    pub fn alphanumeric(mut self, value: &str, field_name: &str) -> Self {
        if let Err(e) = validate_alphanumeric(value, field_name) {
            self.result.add_error(e);
        }
        self
    }

    pub fn slug(mut self, value: &str, field_name: &str) -> Self {
        if let Err(e) = validate_slug(value, field_name) {
            self.result.add_error(e);
        }
        self
    }

    pub fn username(mut self, value: &str) -> Self {
        if let Err(e) = validate_username(value) {
            self.result.add_error(e);
        }
        self
    }

    pub fn password(mut self, value: &str) -> Self {
        if let Err(e) = validate_password_strength(value) {
            self.result.add_error(e);
        }
        self
    }

    pub fn no_html(mut self, value: &str, field_name: &str) -> Self {
        if let Err(e) = validate_no_html(value, field_name) {
            self.result.add_error(e);
        }
        self
    }

    pub fn no_script(mut self, value: &str, field_name: &str) -> Self {
        if let Err(e) = validate_no_script_injection(value, field_name) {
            self.result.add_error(e);
        }
        self
    }

    pub fn custom<F>(mut self, validation_fn: F) -> Self
    where
        F: FnOnce() -> Option<ValidationError>,
    {
        if let Some(error) = validation_fn() {
            self.result.add_error(error);
        }
        self
    }

    pub fn ssrf_safe_url(mut self, value: &str) -> Self {
        if let Err(e) = validate_url_ssrf(value) {
            self.result.add_error(e);
        }
        self
    }

    pub fn validate(self) -> Result<(), ValidationResult> {
        if self.result.is_valid() {
            Ok(())
        } else {
            Err(self.result)
        }
    }

    pub fn result(self) -> ValidationResult {
        self.result
    }
}
impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}
