use async_trait::async_trait;
use crate::core::models::{Message, ToolDefinition, TokenUsage};
use serde_json::Value;

pub type ModelResult<T> = Result<T, ModelError>;

#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Network error: {0}")]
    NetworkError(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    #[error("Stream error: {0}")]
    StreamError(String),
}

#[derive(Debug, Clone)]
pub struct ModelRequest {
    pub messages: Vec<Message>,
    pub model: String,
    pub tools: Vec<ToolDefinition>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone)]
pub enum StreamChunk {
    TextDelta { text: String },
    ReasoningDelta { text: String },
    ToolCallStart { id: String, name: String },
    ToolCallArgsDelta { id: String, args_json: String },
    ToolCallComplete { id: String },
    Done { usage: TokenUsage },
}

#[async_trait]
pub trait ModelClient: Send + Sync {
    async fn stream_completion(
        &self,
        request: ModelRequest,
    ) -> ModelResult<Box<dyn futures::Stream<Item = ModelResult<StreamChunk>> + Send + Unpin>>;

    fn supports_reasoning(&self) -> bool {
        false
    }

    fn supports_vision(&self) -> bool {
        false
    }
}
