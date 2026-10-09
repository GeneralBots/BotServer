use super::*;

impl std::fmt::Display for Distance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Distance::Cosine => write!(f, "Cosine"),
            Distance::Euclid => write!(f, "Euclid"),
            Distance::Dot => write!(f, "Dot"),
            Distance::Manhattan => write!(f, "Manhattan"),
        }
    }
}
