use serde::{Deserialize, Serialize};

/// 任务节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub dependencies: Vec<String>, // 依赖的任务 ID
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// 任务分解器
pub struct TaskDecomposer;

impl TaskDecomposer {
    /// 使用 LLM 分解任务
    pub async fn decompose(
        provider: &dyn crate::providers::LlmProvider,
        user_task: &str,
        model: &str,
    ) -> Result<Vec<Task>, String> {
        let prompt = format!(
            r#"将以下任务分解为可独立执行的子任务。

用户任务：{}

要求：
1. 每个子任务应该是原子操作（使用单个工具即可完成）
2. 识别任务间的依赖关系
3. 可以并行执行的任务不应有依赖关系

请以 JSON 格式返回，格式如下：
```json
[
  {{"id": "1", "description": "子任务描述", "dependencies": []}},
  {{"id": "2", "description": "子任务描述", "dependencies": ["1"]}}
]
```

只返回 JSON，不要其他内容。"#,
            user_task
        );

        let messages = vec![crate::providers::types::UnifiedMessage {
            role: crate::providers::types::MessageRole::User,
            content: vec![crate::providers::types::ContentBlock::Text { text: prompt }],
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }];

        let response = provider.chat_sync(messages, model).await
            .map_err(|e| format!("LLM 调用失败: {}", e))?;

        // 提取 JSON
        let json_str = if let Some(start) = response.find('[') {
            if let Some(end) = response.rfind(']') {
                &response[start..=end]
            } else {
                &response
            }
        } else {
            &response
        };

        let task_defs: Vec<serde_json::Value> = serde_json::from_str(json_str)
            .map_err(|e| format!("解析任务失败: {}", e))?;

        let tasks = task_defs
            .into_iter()
            .map(|t| Task {
                id: t["id"].as_str().unwrap_or("").to_string(),
                description: t["description"].as_str().unwrap_or("").to_string(),
                status: TaskStatus::Pending,
                dependencies: t["dependencies"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default(),
                result: None,
            })
            .collect();

        Ok(tasks)
    }

    /// 获取可执行的任务（无依赖或依赖已完成）
    pub fn get_ready_tasks(tasks: &[Task]) -> Vec<String> {
        tasks
            .iter()
            .filter(|t| {
                t.status == TaskStatus::Pending
                    && t.dependencies.iter().all(|dep_id| {
                        tasks
                            .iter()
                            .find(|t2| t2.id == *dep_id)
                            .map(|t2| t2.status == TaskStatus::Completed)
                            .unwrap_or(false)
                    })
            })
            .map(|t| t.id.clone())
            .collect()
    }
}

#[cfg(test)]
#[path = "task_decomposer_tests.rs"]
mod tests;

