use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TurnStatus {
    Running,
    Completed,
    Interrupted,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub id: String,
    pub thread_id: String,
    pub items: Vec<TurnItem>,
    pub status: TurnStatus,
    pub model: String,
    pub usage: TokenUsage,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TurnItem {
    UserMessage {
        text: String,
        attachments: Vec<String>,
    },
    AssistantText {
        text: String,
    },
    AssistantReasoning {
        content: String,
    },
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
    ToolResult {
        call_id: String,
        result: String,
        error: Option<String>,
    },
    Compaction {
        summary: String,
        removed_count: usize,
        digest: String,
    },
    Approval {
        id: String,
        prompt: String,
        tool_name: String,
        args: Value,
        status: ApprovalStatus,
    },
    Error {
        code: String,
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub total_tokens: u64,
}

impl Turn {
    pub fn new(thread_id: String, model: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            thread_id,
            items: Vec::new(),
            status: TurnStatus::Running,
            model,
            usage: TokenUsage::default(),
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn add_item(&mut self, item: TurnItem) {
        self.items.push(item);
    }

    pub fn complete(&mut self, usage: TokenUsage) {
        self.status = TurnStatus::Completed;
        self.usage = usage;
        self.completed_at = Some(Utc::now());
    }
}
