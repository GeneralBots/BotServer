use super::*;

impl PasswordStrength {
    pub fn score(&self) -> u8 {
        match self {
            Self::VeryWeak => 0,
            Self::Weak => 1,
            Self::Fair => 2,
            Self::Strong => 3,
            Self::VeryStrong => 4,
        }
    }

    pub fn is_acceptable(&self) -> bool {
        matches!(self, Self::Fair | Self::Strong | Self::VeryStrong)
    }
}
