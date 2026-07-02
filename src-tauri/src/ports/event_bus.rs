use serde::{Deserialize, Serialize};
use crate::core::models::{TurnItem, ApprovalStatus};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    TurnStarted { turn_id: String, model: String },
    TextDelta { text: String },
    ReasoningDelta { text: String },
    ToolCallStarted { id: String, name: String },
    ToolCallArgsDelta { id: String, args_json: String },
    ToolCallComplete { id: String },
    ToolResultReceived { call_id: String, result: String, error: Option<String> },
    ItemAdded { item: TurnItem },
    TurnCompleted { usage: crate::core::models::TokenUsage },
    TurnInterrupted,
    Error { code: String, message: String },
    ApprovalRequested { id: String, tool_name: String, prompt: String },
    ApprovalResolved { id: String, status: ApprovalStatus },
}

pub trait EventBus: Send + Sync {
    fn emit(&self, thread_id: &str, event: StreamEvent);
    fn subscribe(&self, thread_id: &str) -> tokio::sync::broadcast::Receiver<StreamEvent>;
}
