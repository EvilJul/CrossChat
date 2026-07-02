use async_trait::async_trait;
use crate::core::models::{ToolDefinition, ToolCall, ToolResult as ToolResultModel};

pub type ToolResult<T> = Result<T, ToolError>;

#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),
    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Timeout")]
    Timeout,
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

#[async_trait]
pub trait ToolHost: Send + Sync {
    async fn execute(&self, call: &ToolCall) -> ToolResult<ToolResultModel>;
    async fn list_tools(&self) -> Vec<ToolDefinition>;
    fn is_tool_allowed(&self, name: &str) -> bool;
}
