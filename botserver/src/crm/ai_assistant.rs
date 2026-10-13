use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIAssistant {
    pub enabled: bool,
    pub suggestions_enabled: bool,
    pub sentiment_analysis: bool,
    pub auto_summary: bool,
    pub model: String,
}

impl Default for AIAssistant {
    fn default() -> Self {
        Self {
            enabled: true,
            suggestions_enabled: true,
            sentiment_analysis: true,
            auto_summary: true,
            model: "default".to_string(),
        }
    }
}

impl AIAssistant {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_model(mut self, model: &str) -> Self {
        self.model = model.to_string();
        self
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn generate_suggestions(&self, conversation: &[String]) -> Vec<String> {
        if !self.enabled || !self.suggestions_enabled || conversation.is_empty() {
            return Vec::new();
        }

        let last_msg = conversation.last().map(|s| s.as_str()).unwrap_or("");
        let lower = last_msg.to_lowercase();

        let mut suggestions = Vec::new();

        if lower.contains("obrigado") || lower.contains("valeu") || lower.contains("thanks") {
            suggestions.push("De nada! Precisa de mais alguma ajuda?".to_string());
            suggestions.push("Fico à disposição!".to_string());
        }

        if lower.contains("problema") || lower.contains("erro") || lower.contains("bug") {
            suggestions.push("Pode me descrever o erro com mais detalhes?".to_string());
            suggestions.push("Vou abrir um ticket de suporte para acompanhar isso.".to_string());
            suggestions.push("Qual a urgência desse problema?".to_string());
        }

        if lower.contains("comprar") || lower.contains("preço") || lower.contains("quanto custa") {
            suggestions.push("Vou verificar os preços disponíveis para você.".to_string());
            suggestions.push("Posso transferir para o time de vendas.".to_string());
        }

        if lower.contains("falar com") || lower.contains("transferir") || lower.contains("humano") {
            suggestions.push("Vou transferir para um atendente humano.".to_string());
            suggestions.push("Posso ajudar com mais informações antes da transferência.".to_string());
        }

        if suggestions.is_empty() {
            suggestions.push("Entendi. Pode me dar mais contexto?".to_string());
            suggestions.push("Como posso ajudar?".to_string());
            suggestions.push("Gostaria de falar com um atendente?".to_string());
        }

        suggestions.truncate(3);
        suggestions
    }

    pub fn analyze_sentiment(&self, text: &str) -> String {
        if !self.enabled || !self.sentiment_analysis {
            return "neutral".to_string();
        }

        let lower = text.to_lowercase();
        let positive_words = [
            "obrigado", "ótimo", "excelente", "bom", "maravilhoso", "perfeito",
            "adoro", "amei", "feliz", "satisfeito", "great", "good", "thanks",
        ];
        let negative_words = [
            "péssimo", "horrível", "ruim", "terrível", "odeio", "frustrado",
            "raiva", "insatisfeito", "lento", "demorado", "bad", "terrible",
            "horrible", "angry", "frustrated",
        ];
        let urgent_words = [
            "urgente", "emergência", "crítico", "critical", "emergency",
            "imediato", "imediatamente", "now", "urgent",
        ];

        let mut score: i32 = 0;
        for w in &urgent_words {
            if lower.contains(w) {
                score -= 3;
            }
        }
        for w in &negative_words {
            if lower.contains(w) {
                score -= 1;
            }
        }
        for w in &positive_words {
            if lower.contains(w) {
                score += 1;
            }
        }

        // Symmetric bands: a single lexicon hit already moves the message off
        // neutral ("Excelente atendimento!" scores 1 and must read positive,
        // the same way one negative word reads negative).
        if score < -2 {
            "very_negative"
        } else if score < 0 {
            "negative"
        } else if score > 0 {
            "positive"
        } else {
            "neutral"
        }
        .to_string()
    }

    pub fn summarize(&self, conversation: &[String]) -> String {
        if !self.enabled || !self.auto_summary || conversation.is_empty() {
            return String::new();
        }

        let total_msgs = conversation.len();
        let customer_msgs: Vec<&String> = conversation.iter().filter(|m| !m.is_empty()).collect();

        if customer_msgs.is_empty() {
            return "No customer messages to summarize.".to_string();
        }

        let first_msg = customer_msgs.first().map(|s| s.as_str()).unwrap_or("");
        let last_msg = customer_msgs.last().map(|s| s.as_str()).unwrap_or("");

        format!(
            "Conversation summary: {total_msgs} messages total. \
            Started with: \"{first}\". \
            Last message: \"{last}\". \
            Sentiment trend: {sentiment}.",
            first = Self::truncate(first_msg, 80),
            last = Self::truncate(last_msg, 80),
            sentiment = self.analyze_sentiment(last_msg),
        )
    }

    fn truncate(s: &str, max: usize) -> String {
        botlib::text::truncate_with_suffix(s, max, "...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestions_thanks() {
        let ai = AIAssistant::new();
        let conv = vec!["Obrigado pela ajuda!".to_string()];
        let suggestions = ai.generate_suggestions(&conv);
        assert!(!suggestions.is_empty());
        assert!(suggestions[0].contains("mais alguma ajuda"));
    }

    #[test]
    fn test_suggestions_error() {
        let ai = AIAssistant::new();
        let conv = vec!["Estou com um problema no sistema.".to_string()];
        let suggestions = ai.generate_suggestions(&conv);
        assert!(suggestions.iter().any(|s| s.contains("ticket")));
    }

    #[test]
    fn test_sentiment_positive() {
        let ai = AIAssistant::new();
        assert_eq!(ai.analyze_sentiment("Excelente atendimento!"), "positive");
    }

    #[test]
    fn test_sentiment_negative() {
        let ai = AIAssistant::new();
        assert_eq!(ai.analyze_sentiment("Péssimo serviço, muito lento!"), "negative");
    }

    #[test]
    fn test_sentiment_urgent() {
        let ai = AIAssistant::new();
        assert_eq!(ai.analyze_sentiment("ISSO É URGENTE!"), "very_negative");
    }

    #[test]
    fn test_summary() {
        let ai = AIAssistant::new();
        let conv = vec![
            "Olá, preciso de ajuda.".to_string(),
            "Estou com um problema.".to_string(),
            "Obrigado, resolveu!".to_string(),
        ];
        let summary = ai.summarize(&conv);
        assert!(summary.contains("3 messages"));
        assert!(summary.contains("Olá"));
    }

    #[test]
    fn test_disabled() {
        let mut ai = AIAssistant::new();
        ai.enabled = false;
        let conv = vec!["Test".to_string()];
        assert!(ai.generate_suggestions(&conv).is_empty());
        assert_eq!(ai.analyze_sentiment("Test"), "neutral");
    }
}
