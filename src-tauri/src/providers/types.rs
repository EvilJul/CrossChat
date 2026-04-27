use serde::{Deserialize, Serialize};

/// 统一消息格式 (不依赖任何特定 Provider)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMessage {
    pub role: MessageRole,
    pub content: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// DeepSeek R1 等模型的思考链内容（需回传给 API）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
}

/// 统一工具调用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// 统一工具定义 (OpenAI 兼容 JSON Schema)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// 流式响应块 (Provider 无关)
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum StreamChunk {
    TextDelta {
        delta: String,
    },
    ToolCallStart {
        id: String,
        name: String,
    },
    ToolCallDelta {
        id: String,
        arguments_delta: String,
    },
    ToolCallEnd {
        id: String,
    },
    Done {
        finish_reason: Option<String>,
    },
    ThinkingDelta {
        delta: String,
    },
    ThinkingDone {},
    Error {
        message: String,
    },
}
