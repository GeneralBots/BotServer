use super::*;

impl From<&str> for ReflectionType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "conversation_quality" | "conversation" | "quality" => Self::ConversationQuality,
            "response_accuracy" | "accuracy" | "response" => Self::ResponseAccuracy,
            "tool_usage" | "tools" => Self::ToolUsage,
            "knowledge_retrieval" | "knowledge" | "retrieval" => Self::KnowledgeRetrieval,
            "performance" | "overall" => Self::Performance,
            _ => Self::Custom(s.to_string()),
        }
    }
}
impl ReflectionType {
    pub fn prompt_template(&self) -> String {
        match self {
            Self::ConversationQuality => r#"Analyze the following conversation and evaluate:
1. User satisfaction indicators (positive/negative sentiment)
2. Conversation flow and coherence
3. Whether user's questions were fully addressed
4. Response tone appropriateness
5. Areas for improvement

Conversation:
{conversation}

Provide your analysis in JSON format:
{
    "score": 0-10,
    "satisfaction_indicators": ["..."],
    "strengths": ["..."],
    "weaknesses": ["..."],
    "improvements": ["..."],
    "patterns_noticed": ["..."]
}"#
            .to_string(),
            Self::ResponseAccuracy => {
                r#"Analyze the accuracy and relevance of responses in this conversation:
1. Were responses factually accurate?
2. Were responses relevant to the questions?
3. Were there any hallucinations or incorrect information?
4. Was the level of detail appropriate?
5. Were sources/references needed but not provided?

Conversation:
{conversation}

Provide your analysis in JSON format:
{
    "accuracy_score": 0-10,
    "relevance_score": 0-10,
    "factual_errors": ["..."],
    "hallucinations": ["..."],
    "missing_information": ["..."],
    "improvements": ["..."]
}"#
                .to_string()
            }
            Self::ToolUsage => r#"Analyze tool usage in this conversation:
1. Were tools used appropriately?
2. Were there missed opportunities to use tools?
3. Did tool outputs meet user needs?
4. Were there any tool errors or failures?
5. Could tools have been combined more effectively?

Conversation:
{conversation}

Tools Available:
{tools}

Provide your analysis in JSON format:
{
    "tool_effectiveness_score": 0-10,
    "tools_used": ["..."],
    "missed_opportunities": ["..."],
    "effective_uses": ["..."],
    "ineffective_uses": ["..."],
    "recommendations": ["..."]
}"#
            .to_string(),
            Self::KnowledgeRetrieval => r#"Analyze knowledge base retrieval in this conversation:
1. Were relevant documents retrieved?
2. Was the context provided to the LLM appropriate?
3. Were there questions that should have used KB but didn't?
4. Was there irrelevant context that confused responses?
5. Were there knowledge gaps identified?

Conversation:
{conversation}

Retrieved Context:
{context}

Provide your analysis in JSON format:
{
    "retrieval_score": 0-10,
    "relevant_retrievals": ["..."],
    "irrelevant_retrievals": ["..."],
    "missed_retrievals": ["..."],
    "knowledge_gaps": ["..."],
    "improvements": ["..."]
}"#
            .to_string(),
            Self::Performance => r#"Provide an overall performance analysis of this conversation:
1. Response quality and helpfulness
2. Efficiency (number of turns to resolve issues)
3. User engagement level
4. Task completion rate
5. Overall effectiveness

Conversation:
{conversation}

Provide your analysis in JSON format:
{
    "overall_score": 0-10,
    "response_quality": 0-10,
    "efficiency": 0-10,
    "engagement": 0-10,
    "task_completion": 0-10,
    "key_insights": ["..."],
    "critical_improvements": ["..."],
    "positive_patterns": ["..."]
}"#
            .to_string(),
            Self::Custom(prompt) => prompt.clone(),
        }
    }
}
