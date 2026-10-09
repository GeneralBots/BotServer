use super::*;

impl Action {
    pub fn description(&self) -> String {
        match self {
            Self::Click(loc) => format!("click on {}", loc.as_selector()),
            Self::SendKeys(text) => format!("send keys: {text}"),
            Self::Pause(dur) => format!("pause for {dur:?}"),
        }
    }
}
