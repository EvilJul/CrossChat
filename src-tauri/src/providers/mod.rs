pub mod types;
pub mod openai_compat;

use async_trait::async_trait;
use std::collections::HashMap;
use tauri::ipc::Channel;
use types::{StreamChunk, ToolDefinition, UnifiedMessage};

/// Provider 类型标识
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum ProviderType {
    OpenAICompat,
    Anthropic,
}

/// Provider 错误类型
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ProviderError {
    #[error("HTTP 请求失败: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("API 错误: {0}")]
    ApiError(String),
    #[error("流式传输中断: {0}")]
    StreamError(String),
}

/// LLM Provider 统一接口
#[async_trait]
#[allow(dead_code)]
pub trait LlmProvider: Send + Sync {
    /// 提供商标识类型
    fn provider_type(&self) -> ProviderType;

    /// 流式聊天补全
    async fn stream_chat(
        &self,
        messages: Vec<UnifiedMessage>,
        tools: Vec<ToolDefinition>,
        model: &str,
        channel: Channel<StreamChunk>,
    ) -> Result<(), ProviderError>;

    /// 非流式聊天 (用于简单场景如拉取模型列表)
    async fn chat_sync(
        &self,
        messages: Vec<UnifiedMessage>,
        model: &str,
    ) -> Result<String, ProviderError>;
}

/// Provider 注册表
#[allow(dead_code)]
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn LlmProvider>>,
}

#[allow(dead_code)]
impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: String, provider: Box<dyn LlmProvider>) {
        self.providers.insert(id, provider);
    }

    pub fn get(&self, id: &str) -> Option<&Box<dyn LlmProvider>> {
        self.providers.get(id)
    }

    pub fn list(&self) -> Vec<&String> {
        self.providers.keys().collect()
    }
}
