use super::*;

impl Default for MockStripeClient {
    fn default() -> Self {
        Self::new()
    }
}
pub(super) fn generate_stripe_id() -> String {
    let uuid = Uuid::new_v4();
    uuid.to_string().replace('-', "")[..24].to_string()
}
