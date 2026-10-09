use super::*;

impl Pin {
    pub fn url(&self) -> String {
        format!("https://www.pinterest.com/pin/{}/", self.id)
    }
}
