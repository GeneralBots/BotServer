use super::*;

impl ActionChain {
    pub const fn new() -> Self {
        Self {
            actions: Vec::new(),
        }
    }

    pub fn click(mut self, locator: Locator) -> Self {
        self.actions.push(Action::Click(locator));
        self
    }

    pub fn send_keys(mut self, text: &str) -> Self {
        self.actions.push(Action::SendKeys(text.to_string()));
        self
    }

    pub fn pause(mut self, duration: Duration) -> Self {
        self.actions.push(Action::Pause(duration));
        self
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }
}
impl Default for ActionChain {
    fn default() -> Self {
        Self::new()
    }
}
