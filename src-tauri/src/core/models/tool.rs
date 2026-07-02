use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub output: String,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

impl ToolResult {
    pub fn success(call_id: String, output: String, execution_time_ms: u64) -> Self {
        Self {
            call_id,
            output,
            error: None,
            execution_time_ms,
        }
    }

    pub fn error(call_id: String, error: String, execution_time_ms: u64) -> Self {
        Self {
            call_id,
            output: String::new(),
            error: Some(error),
            execution_time_ms,
        }
    }
}
