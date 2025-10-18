use crate::basic::keywords::add_tool::get_session_tools;
use crate::kb::embeddings::search_similar;
use crate::shared::models::UserSession;
use crate::shared::state::AppState;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::sync::Arc;

/// Answer modes for the bot
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AnswerMode {
    Direct = 0,        // Direct LLM response
    WithTools = 1,     // LLM with tool calling
    DocumentsOnly = 2, // Search KB documents only, no LLM
    WebSearch = 3,     // Include web search results
    Mixed = 4,         // Use tools stack from ADD_TOOL and KB from session
}

impl AnswerMode {
    pub fn from_i32(value: i32) -> Self {
        match value {
            0 => Self::Direct,
            1 => Self::WithTools,
            2 => Self::DocumentsOnly,
            3 => Self::WebSearch,
            4 => Self::Mixed,
            _ => Self::Direct,
        }
    }
}

/// Context from KB documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentContext {
    pub source: String,
    pub content: String,
    pub score: f32,
    pub collection_name: String,
}

/// Context from tools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolContext {
    pub tool_name: String,
    pub description: String,
    pub endpoint: String,
}

/// Enhanced prompt with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedPrompt {
    pub original_query: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub document_contexts: Vec<DocumentContext>,
    pub available_tools: Vec<ToolContext>,
    pub answer_mode: AnswerMode,
}

/// Prompt processor that enhances queries with KB and tool context
pub struct PromptProcessor {
    state: Arc<AppState>,
}

impl PromptProcessor {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Process a user query and enhance it with context
    pub async fn process_query(
        &self,
        session: &UserSession,
        query: &str,
    ) -> Result<EnhancedPrompt, Box<dyn Error + Send + Sync>> {
        let answer_mode = AnswerMode::from_i32(session.answer_mode);

        info!(
            "Processing query in {:?} mode: {}",
            answer_mode,
            query.chars().take(50).collect::<String>()
        );

        match answer_mode {
            AnswerMode::Direct => self.process_direct(query).await,
            AnswerMode::WithTools => self.process_with_tools(session, query).await,
            AnswerMode::DocumentsOnly => self.process_documents_only(session, query).await,
            AnswerMode::WebSearch => self.process_web_search(session, query).await,
            AnswerMode::Mixed => self.process_mixed(session, query).await,
        }
    }

    /// Direct mode: no additional context
    async fn process_direct(
        &self,
        query: &str,
    ) -> Result<EnhancedPrompt, Box<dyn Error + Send + Sync>> {
        Ok(EnhancedPrompt {
            original_query: query.to_string(),
            system_prompt: "You are a helpful AI assistant.".to_string(),
            user_prompt: query.to_string(),
            document_contexts: Vec::new(),
            available_tools: Vec::new(),
            answer_mode: AnswerMode::Direct,
        })
    }

    /// With tools mode: include available tools
    async fn process_with_tools(
        &self,
        session: &UserSession,
        query: &str,
    ) -> Result<EnhancedPrompt, Box<dyn Error + Send + Sync>> {
        let tools = self.get_available_tools(session).await?;

        let system_prompt = if tools.is_empty() {
            "You are a helpful AI assistant.".to_string()
        } else {
            format!(
                "You are a helpful AI assistant with access to the following tools:\n{}",
                self.format_tools_for_prompt(&tools)
            )
        };

        Ok(EnhancedPrompt {
            original_query: query.to_string(),
            system_prompt,
            user_prompt: query.to_string(),
            document_contexts: Vec::new(),
            available_tools: tools,
            answer_mode: AnswerMode::WithTools,
        })
    }

    /// Documents only mode: search KB and use documents to answer
    async fn process_documents_only(
        &self,
        session: &UserSession,
        query: &str,
    ) -> Result<EnhancedPrompt, Box<dyn Error + Send + Sync>> {
        let documents = self.search_kb_documents(session, query, 5).await?;

        let system_prompt = "You are a helpful AI assistant. Answer the user's question based ONLY on the provided documents. If the documents don't contain relevant information, say so.".to_string();

        let user_prompt = if documents.is_empty() {
            format!("Question: {}\n\nNo relevant documents found.", query)
        } else {
            format!(
                "Question: {}\n\nRelevant documents:\n{}",
                query,
                self.format_documents_for_prompt(&documents)
            )
        };

        Ok(EnhancedPrompt {
            original_query: query.to_string(),
            system_prompt,
            user_prompt,
            document_contexts: documents,
            available_tools: Vec::new(),
            answer_mode: AnswerMode::DocumentsOnly,
        })
    }

    /// Web search mode: include web search results
    async fn process_web_search(
        &self,
        _session: &UserSession,
        query: &str,
    ) -> Result<EnhancedPrompt, Box<dyn Error + Send + Sync>> {
        // TODO: Implement web search integration
        debug!("Web search mode not fully implemented yet");
        self.process_direct(query).await
    }

    /// Mixed mode: combine KB documents and tools
    async fn process_mixed(
        &self,
        session: &UserSession,
        query: &str,
    ) -> Result<EnhancedPrompt, Box<dyn Error + Send + Sync>> {
        // Get both documents and tools
        let documents = self.search_kb_documents(session, query, 3).await?;
        let tools = self.get_available_tools(session).await?;

        let mut system_parts = vec!["You are a helpful AI assistant.".to_string()];

        if !documents.is_empty() {
            system_parts.push(
                "Use the provided documents as knowledge base to answer questions.".to_string(),
            );
        }

        if !tools.is_empty() {
            system_parts.push(format!(
                "You have access to the following tools:\n{}",
                self.format_tools_for_prompt(&tools)
            ));
        }

        let system_prompt = system_parts.join("\n\n");

        let user_prompt = if documents.is_empty() {
            query.to_string()
        } else {
            format!(
                "Context from knowledge base:\n{}\n\nQuestion: {}",
                self.format_documents_for_prompt(&documents),
                query
            )
        };

        Ok(EnhancedPrompt {
            original_query: query.to_string(),
            system_prompt,
            user_prompt,
            document_contexts: documents,
            available_tools: tools,
            answer_mode: AnswerMode::Mixed,
        })
    }

    /// Search KB documents for a query
    async fn search_kb_documents(
        &self,
        session: &UserSession,
        query: &str,
        limit: usize,
    ) -> Result<Vec<DocumentContext>, Box<dyn Error + Send + Sync>> {
        // Get active KB collections from session context
        let collections = self.get_active_collections(session).await?;

        if collections.is_empty() {
            debug!("No active KB collections for session");
            return Ok(Vec::new());
        }

        let mut all_results = Vec::new();

        // Search in each collection
        for collection_name in collections {
            debug!("Searching in collection: {}", collection_name);

            match search_similar(&self.state, &collection_name, query, limit).await {
                Ok(results) => {
                    for result in results {
                        all_results.push(DocumentContext {
                            source: result.file_path,
                            content: result.chunk_text,
                            score: result.score,
                            collection_name: collection_name.clone(),
                        });
                    }
                }
                Err(e) => {
                    error!("Failed to search collection {}: {}", collection_name, e);
                }
            }
        }

        // Sort by score and limit
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        all_results.truncate(limit);

        info!("Found {} relevant documents", all_results.len());

        Ok(all_results)
    }

    /// Get active KB collections from session context
    async fn get_active_collections(
        &self,
        session: &UserSession,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
        let mut collections = Vec::new();

        // Check for active_kb_collection in context_data
        if let Some(active_kb) = session.context_data.get("active_kb_collection") {
            if let Some(name) = active_kb.as_str() {
                let collection_name = format!("kb_{}_{}", session.bot_id, name);
                collections.push(collection_name);
            }
        }

        // Check for temporary website collections
        if let Some(temp_website) = session.context_data.get("temporary_website_collection") {
            if let Some(name) = temp_website.as_str() {
                collections.push(name.to_string());
            }
        }

        // Check for additional collections from ADD_KB
        if let Some(additional) = session.context_data.get("additional_kb_collections") {
            if let Some(arr) = additional.as_array() {
                for item in arr {
                    if let Some(name) = item.as_str() {
                        let collection_name = format!("kb_{}_{}", session.bot_id, name);
                        collections.push(collection_name);
                    }
                }
            }
        }

        Ok(collections)
    }

    /// Get available tools from session context
    async fn get_available_tools(
        &self,
        session: &UserSession,
    ) -> Result<Vec<ToolContext>, Box<dyn Error + Send + Sync>> {
        let mut tools = Vec::new();

        // Check for tools in session context
        if let Some(tools_data) = session.context_data.get("available_tools") {
            if let Some(arr) = tools_data.as_array() {
                for item in arr {
                    if let (Some(name), Some(desc), Some(endpoint)) = (
                        item.get("name").and_then(|v| v.as_str()),
                        item.get("description").and_then(|v| v.as_str()),
                        item.get("endpoint").and_then(|v| v.as_str()),
                    ) {
                        tools.push(ToolContext {
                            tool_name: name.to_string(),
                            description: desc.to_string(),
                            endpoint: endpoint.to_string(),
                        });
                    }
                }
            }
        }

        // Load all tools associated with this session from session_tool_associations
        if let Ok(mut conn) = self.state.conn.lock() {
            match get_session_tools(&mut *conn, &session.id) {
                Ok(session_tools) => {
                    info!(
                        "Loaded {} tools from session_tool_associations for session {}",
                        session_tools.len(),
                        session.id
                    );

                    for tool_name in session_tools {
                        // Add the tool if not already in list
                        if !tools.iter().any(|t| t.tool_name == tool_name) {
                            tools.push(ToolContext {
                                tool_name: tool_name.clone(),
                                description: format!("Tool: {}", tool_name),
                                endpoint: format!("/default/{}", tool_name),
                            });
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to load session tools: {}", e);
                }
            }
        } else {
            error!("Failed to acquire database lock for loading session tools");
        }

        // Also check for legacy current_tool (backward compatibility)
        if let Some(current_tool) = &session.current_tool {
            // Add the current tool if not already in list
            if !tools.iter().any(|t| &t.tool_name == current_tool) {
                tools.push(ToolContext {
                    tool_name: current_tool.clone(),
                    description: format!("Legacy tool: {}", current_tool),
                    endpoint: format!("/default/{}", current_tool),
                });
            }
        }

        debug!("Found {} available tools", tools.len());

        Ok(tools)
    }

    /// Format documents for inclusion in prompt
    fn format_documents_for_prompt(&self, documents: &[DocumentContext]) -> String {
        documents
            .iter()
            .enumerate()
            .map(|(idx, doc)| {
                format!(
                    "[Document {}] (Source: {}, Relevance: {:.2})\n{}",
                    idx + 1,
                    doc.source,
                    doc.score,
                    doc.content.chars().take(500).collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Format tools for inclusion in prompt
    fn format_tools_for_prompt(&self, tools: &[ToolContext]) -> String {
        tools
            .iter()
            .map(|tool| format!("- {}: {}", tool.tool_name, tool.description))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_answer_mode_from_i32() {
        assert_eq!(AnswerMode::from_i32(0), AnswerMode::Direct);
        assert_eq!(AnswerMode::from_i32(1), AnswerMode::WithTools);
        assert_eq!(AnswerMode::from_i32(2), AnswerMode::DocumentsOnly);
        assert_eq!(AnswerMode::from_i32(3), AnswerMode::WebSearch);
        assert_eq!(AnswerMode::from_i32(4), AnswerMode::Mixed);
        assert_eq!(AnswerMode::from_i32(99), AnswerMode::Direct); // Default
    }

    #[test]
    fn test_format_documents() {
        let processor = PromptProcessor::new(Arc::new(AppState::default()));

        let docs = vec![
            DocumentContext {
                source: "test.pdf".to_string(),
                content: "This is test content".to_string(),
                score: 0.95,
                collection_name: "test_collection".to_string(),
            },
            DocumentContext {
                source: "another.pdf".to_string(),
                content: "More content here".to_string(),
                score: 0.85,
                collection_name: "test_collection".to_string(),
            },
        ];

        let formatted = processor.format_documents_for_prompt(&docs);

        assert!(formatted.contains("[Document 1]"));
        assert!(formatted.contains("[Document 2]"));
        assert!(formatted.contains("test.pdf"));
        assert!(formatted.contains("This is test content"));
    }

    #[test]
    fn test_format_tools() {
        let processor = PromptProcessor::new(Arc::new(AppState::default()));

        let tools = vec![
            ToolContext {
                tool_name: "enrollment".to_string(),
                description: "Enroll a user".to_string(),
                endpoint: "/default/enrollment".to_string(),
            },
            ToolContext {
                tool_name: "pricing".to_string(),
                description: "Get product pricing".to_string(),
                endpoint: "/default/pricing".to_string(),
            },
        ];

        let formatted = processor.format_tools_for_prompt(&tools);

        assert!(formatted.contains("enrollment"));
        assert!(formatted.contains("Enroll a user"));
        assert!(formatted.contains("pricing"));
    }
}
