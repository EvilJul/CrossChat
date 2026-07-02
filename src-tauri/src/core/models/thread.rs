use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThreadStatus {
    Active,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThreadMode {
    Chat,
    Write,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub title: String,
    pub workspace_path: Option<PathBuf>,
    pub status: ThreadStatus,
    pub mode: ThreadMode,
    pub goal: Option<ThreadGoal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // 是否置顶：置顶会话在列表中优先展示
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadGoal {
    pub description: String,
    pub status: GoalStatus,
    pub token_budget: Option<u64>,
    pub elapsed_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GoalStatus {
    Active,
    Paused,
    Blocked,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadTodo {
    pub id: String,
    pub thread_id: String,
    pub text: String,
    pub completed: bool,
    pub plan_linked: bool,
    pub created_at: DateTime<Utc>,
}

impl Thread {
    pub fn new(title: String, workspace_path: Option<PathBuf>, mode: ThreadMode) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title,
            workspace_path,
            status: ThreadStatus::Active,
            mode,
            goal: None,
            created_at: now,
            updated_at: now,
            pinned: false,
        }
    }
}
