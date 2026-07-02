use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: Vec<ContentPart>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    Text { text: String },
    Image { url: String, detail: Option<String> },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}

impl Message {
    pub fn system(text: String) -> Self {
        Self {
            role: MessageRole::System,
            content: vec![ContentPart::Text { text }],
        }
    }

    pub fn user(text: String) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![ContentPart::Text { text }],
        }
    }

    pub fn assistant(text: String) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: vec![ContentPart::Text { text }],
        }
    }
}
