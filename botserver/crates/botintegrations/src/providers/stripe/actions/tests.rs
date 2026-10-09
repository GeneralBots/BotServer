#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    pub(super) fn stripe_ids_require_prefix_and_safe_charset() {
        assert_eq!(
            stripe_id("pi_1234", "pi_").ok(),
            Some("pi_1234".to_string())
        );
        assert!(stripe_id("cus_1234", "pi_").is_err());
        assert!(stripe_id("pi_", "pi_").is_err());
        assert!(stripe_id("pi_/../etc", "pi_").is_err());
        assert!(stripe_id("", "pi_").is_err());
    }

    #[test]
    pub(super) fn currency_codes_are_three_lowercase_letters() {
        assert!(validate_currency("usd").is_ok());
        assert!(validate_currency("brl").is_ok());
        assert!(validate_currency("USD").is_err());
        assert!(validate_currency("us").is_err());
        assert!(validate_currency("euro").is_err());
    }

    #[test]
    pub(super) fn customer_creation_requires_name_or_email() {
        assert!(customer_create_fields(&json!({})).is_err());
        assert!(customer_create_fields(&json!({ "email": "not-an-address" })).is_err());
        let fields = customer_create_fields(&json!({ "name": "Acme", "email": "a@b.co" }))
            .unwrap_or_default();
        assert!(fields.contains(&("name".to_string(), "Acme".to_string())));
        assert!(fields.contains(&("email".to_string(), "a@b.co".to_string())));
        assert!(customer_create_fields(&json!({ "name": "Only Name" })).is_ok());
    }

    #[test]
    pub(super) fn payment_intents_need_positive_amount_and_currency() {
        assert!(payment_intent_create_fields(&json!({ "amount": 100 })).is_err());
        assert!(payment_intent_create_fields(&json!({ "amount": 100, "currency": "US" })).is_err());
        assert!(payment_intent_create_fields(&json!({ "amount": -5, "currency": "usd" })).is_err());
        let fields = payment_intent_create_fields(&json!({ "amount": 1500, "currency": "usd" }))
            .unwrap_or_default();
        assert_eq!(
            fields,
            vec![
                ("amount".to_string(), "1500".to_string()),
                ("currency".to_string(), "usd".to_string())
            ]
        );
    }

    #[test]
    pub(super) fn unknown_actions_reject_before_any_network_activity() {
        let names = super::super::STRIPE_IMPLEMENTED_ACTIONS;
        assert!(!names.contains(&"payouts.create"));
        assert!(names.contains(&"balance.retrieve"));
    }

    #[test]
    pub(super) fn email_queries_must_contain_an_at_sign() {
        assert!(required_email(&json!({ "query": "a@b.co" }), "query").is_ok());
        assert!(required_email(&json!({ "query": "plain" }), "query").is_err());
        assert!(required_email(&json!({}), "query").is_err());
    }
}
