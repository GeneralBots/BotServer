use super::*;

impl std::fmt::Display for ActionResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Denied => "denied",
            Self::Error => "error",
        };
        write!(f, "{s}")
    }
}
impl std::str::FromStr for ActionResult {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(Self::Success),
            "failure" => Ok(Self::Failure),
            "denied" => Ok(Self::Denied),
            "error" => Ok(Self::Error),
            _ => Err(format!("Unknown result: {s}")),
        }
    }
}
