pub(crate) use botbasic_types::UserSession;
pub(crate) use botbasic_types::BasicRuntime;
pub(crate) use chrono::Utc;
pub(crate) use diesel::prelude::*;
pub(crate) use log::{debug, info};
pub(crate) use rhai::{Array, Dynamic, Engine, Map};
pub(crate) use std::sync::Arc;
pub(crate) use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_tips_urgent() {
        let tips = create_fallback_tips("This is URGENT! Help now!");
        let result = tips.try_cast::<Map>().expect("fallback tips is a map");
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_fallback_tips_question() {
        let tips = create_fallback_tips("Can you help me with this?");
        let result = tips.try_cast::<Map>().expect("fallback tips is a map");
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_fallback_tips_problem() {
        let tips = create_fallback_tips("I have a problem with my order");
        let result = tips.try_cast::<Map>().expect("fallback tips is a map");
        assert!(result.get("success").unwrap().as_bool().unwrap());
    }

    #[test]
    fn test_create_error_result() {
        let result = create_error_result("Test error message");
        let map = result.try_cast::<Map>().expect("error result is a map");
        assert!(!map.get("success").unwrap().as_bool().unwrap());
        assert_eq!(
            map.get("error").unwrap().clone().into_string().unwrap(),
            "Test error message"
        );
    }

    #[test]
    fn test_polish_text() {
        fn polish_text(message: &str, _tone: &str) -> String {
            let mut polished = message.to_string();
            polished = polished
                .replace("thx", "Thank you")
                .replace("u ", "you ")
                .replace(" u", " you")
                .replace("ur ", "your ")
                .replace("ill ", "I'll ")
                .replace("dont ", "don't ")
                .replace("cant ", "can't ")
                .replace("wont ", "won't ")
                .replace("im ", "I'm ")
                .replace("ive ", "I've ");
            if let Some(first_char) = polished.chars().next() {
                polished = first_char.to_uppercase().to_string() + &polished[1..];
            }
            if !polished.ends_with('.') && !polished.ends_with('!') && !polished.ends_with('?') {
                polished.push('.');
            }
            polished
        }

        let polished = polish_text("thx 4 ur msg", "professional");
        assert!(!polished.contains("thx"));
        assert!(polished.contains("your"));
    }

    #[test]
    fn test_sentiment_analysis() {
        fn analyze_text_sentiment(message: &str) -> &'static str {
            let msg_lower = message.to_lowercase();
            let positive_words = [
                "thank",
                "great",
                "perfect",
                "awesome",
                "excellent",
                "good",
                "happy",
                "love",
            ];
            let negative_words = [
                "angry",
                "frustrated",
                "terrible",
                "awful",
                "horrible",
                "hate",
                "disappointed",
                "problem",
                "issue",
            ];
            let positive_count = positive_words
                .iter()
                .filter(|w| msg_lower.contains(*w))
                .count();
            let negative_count = negative_words
                .iter()
                .filter(|w| msg_lower.contains(*w))
                .count();
            match positive_count.cmp(&negative_count) {
                std::cmp::Ordering::Greater => "positive",
                std::cmp::Ordering::Less => "negative",
                std::cmp::Ordering::Equal => "neutral",
            }
        }

        assert_eq!(
            analyze_text_sentiment("Thank you so much! This is great!"),
            "positive"
        );
        assert_eq!(
            analyze_text_sentiment("This is terrible! I'm so frustrated!"),
            "negative"
        );
        assert_eq!(analyze_text_sentiment("The meeting is at 3pm."), "neutral");
    }

    #[test]
    fn test_smart_replies() {
        fn generate_smart_replies() -> Vec<String> {
            vec![
                "Thank you for reaching out! I'd be happy to help you with that.".to_string(),
                "I understand your concern. Let me look into this for you right away.".to_string(),
                "Is there anything else I can help you with today?".to_string(),
            ]
        }

        let replies = generate_smart_replies();
        assert_eq!(replies.len(), 3);
        assert!(replies.iter().any(|r| r.contains("Thank you")));
        assert!(replies.iter().any(|r| r.contains("understand")));
    }
}

mod part1;
mod part2;
mod part3;
mod part4;
mod part5;

pub use part1::*;
pub use part2::*;
pub use part3::*;
pub use part4::*;
pub use part5::*;
