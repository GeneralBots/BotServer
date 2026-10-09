use super::*;

impl UiOp {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "open" => Some(Self::Open),
            "click" => Some(Self::Click),
            "fill" => Some(Self::Fill),
            "select" => Some(Self::Select),
            "submit" => Some(Self::Submit),
            "wait" => Some(Self::Wait),
            "cell" => Some(Self::Cell),
            _ => None,
        }
    }
}
