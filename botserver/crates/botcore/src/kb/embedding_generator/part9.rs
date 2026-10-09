use super::*;

impl EmailLike for SimpleEmail {
    fn subject(&self) -> &str {
        &self.subject
    }
    fn from(&self) -> &str {
        &self.from
    }
    fn to(&self) -> &str {
        &self.to
    }
    fn body(&self) -> &str {
        &self.body
    }
}
