pub mod react;
pub mod task_decomposer;
pub mod tool_registry;

use crate::mcp::global_mcp;
use crate::memory::{global_memory, Memory};
use crate::providers::types::{ChatSyncResult, ContentBlock, MessageRole, StreamChunk, UnifiedMessage};
use crate::providers::LlmProvider;
use crate::tools;
use serde::{Deserialize, Serialize};
use task_decomposer::{Task, TaskDecomposer, TaskStatus};
use tool_registry::ToolRegistry;

/// Agent 执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResult {
    pub success: bool,
    pub final_answer: String,
    pub iterations: usize,
    pub steps: Vec<AgentStep>,
}

/// Agent 执行步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    pub iteration: usize,
    pub thought: String,
    pub action: Option<AgentAction>,
    pub observation: Option<String>,
}

/// Agent 行动
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: String,
    pub success: bool,
}

/// Agent 配置
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_iterations: usize,
    pub work_dir: String,
    pub enable_self_healing: bool,
    pub enable_task_decomposition: bool, // 启用任务分解
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 15,
            work_dir: ".".into(),
            enable_self_healing: true,
            enable_task_decomposition: true,
        }
    }
}

/// 核心 Agent 结构
pub struct Agent {
    config: AgentConfig,
    tool_registry: ToolRegistry,
    context: Vec<UnifiedMessage>,
    steps: Vec<AgentStep>,
}

impl Agent {
    pub fn new(config: AgentConfig, tool_registry: ToolRegistry) -> Self {
        Self {
            config,
            tool_registry,
            context: Vec::new(),
            steps: Vec::new(),
        }
    }

    /// 运行 Agent（ReAct 循环）
    pub async fn run(
        &mut self,
        provider: &dyn LlmProvider,
        initial_messages: Vec<UnifiedMessage>,
        model: &str,
    ) -> Result<AgentResult, String> {
        self.context = initial_messages;

        for iteration in 0..self.config.max_iterations {
            let tool_defs = self.tool_registry.get_all_definitions();

            // 调用 LLM 思考并决定行动
            let result = provider
                .chat_sync_with_tools(self.context.clone(), &tool_defs, model)
                .await
                .map_err(|e| format!("LLM 调用失败: {}", e))?;

            match result {
                ChatSyncResult::Content(text) => {
                    // LLM 给出最终答案
                    return Ok(AgentResult {
                        success: true,
                        final_answer: text,
                        iterations: iteration + 1,
                        steps: self.steps.clone(),
                    });
                }
                ChatSyncResult::ToolCalls { calls, reasoning } => {
                    // 记录思考过程
                    let thought = reasoning.clone().unwrap_or_else(|| "执行工具调用".into());

                    // 执行所有工具调用
                    let mut actions = Vec::new();
                    for tc in &calls {
                        let result = self.tool_registry
                            .execute(&tc.name, &tc.arguments, &self.config.work_dir)
                            .await;

                        actions.push(AgentAction {
                            tool_name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                            result: result.content.clone(),
                            success: result.success,
                        });

                        // 将工具结果添加到上下文
                        self.context.push(UnifiedMessage {
                            role: MessageRole::Tool,
                            content: vec![ContentBlock::Text {
                                text: result.content.clone(),
                            }],
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                            reasoning_content: None,
                        });
                    }

                    // 记录步骤
                    let observation = actions.iter()
                        .map(|a| format!("{}: {}", a.tool_name, a.result))
                        .collect::<Vec<_>>()
                        .join("\n");

                    self.steps.push(AgentStep {
                        iteration,
                        thought,
                        action: actions.into_iter().next(),
                        observation: Some(observation),
                    });

                    // 添加 assistant 消息到上下文
                    self.context.push(UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: vec![],
                        tool_calls: Some(calls),
                        tool_call_id: None,
                        reasoning_content: reasoning,
                    });
                }
            }
        }

        Err(format!("达到最大迭代次数 ({})", self.config.max_iterations))
    }

    /// 运行 Agent 并实时流式输出（支持任务分解）
    pub async fn run_with_stream<T: crate::commands::chat::StreamSender + Clone>(
        &mut self,
        provider: &dyn LlmProvider,
        initial_messages: Vec<UnifiedMessage>,
        model: &str,
        channel: T,
    ) -> Result<AgentResult, String> {
        self.context = initial_messages;

        // 提取用户任务
        let user_task = self.context.iter()
            .find(|m| matches!(m.role, MessageRole::User))
            .and_then(|m| m.content.iter().find_map(|c| {
                if let ContentBlock::Text { text } = c { Some(text.clone()) } else { None }
            }))
            .unwrap_or_default();

        // 任务分解（如果启用且任务复杂）
        if self.config.enable_task_decomposition && is_complex_task(&user_task) {
            match TaskDecomposer::decompose(provider, &user_task, model).await {
                Ok(tasks) if tasks.len() > 1 => {
                    return self.run_with_decomposed_tasks(provider, tasks, model, channel, &user_task).await;
                }
                _ => {
                    // 分解失败或无子任务，回退到标准流程
                }
            }
        }

        // 标准执行流程（无分解）
        self.run_standard(provider, &user_task, model, channel).await
    }

    /// 标准执行流程（无任务分解）
    async fn run_standard<T: crate::commands::chat::StreamSender + Clone>(
        &mut self,
        provider: &dyn LlmProvider,
        user_task: &str,
        model: &str,
        channel: T,
    ) -> Result<AgentResult, String> {
        // 搜索相似记忆
        if !user_task.is_empty() {
            if let Ok(memories) = global_memory().search(user_task, 3) {
                if !memories.is_empty() {
                    let memory_context = format!(
                        "[记忆检索 — 找到 {} 条相似任务的解决方案]\n\n{}",
                        memories.len(),
                        memories.iter().map(|m| format!(
                            "任务: {}\n解决方案: {}\n使用工具: {}\n",
                            m.task, m.solution, m.tools_used
                        )).collect::<Vec<_>>().join("\n---\n")
                    );
                    self.context.insert(0, UnifiedMessage {
                        role: MessageRole::User,
                        content: vec![ContentBlock::Text { text: memory_context }],
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
            }
        }

        let mut tools_used = Vec::new();

        for iteration in 0..self.config.max_iterations {
            let tool_defs = self.tool_registry.get_all_definitions();

            // 发送进度提示，避免前端长时间无反馈
            let _ = channel.send(StreamChunk::StatusDelta {
                message: format!("思考中 (第 {} 轮)", iteration + 1),
            });

            let result = provider
                .chat_sync_with_tools(self.context.clone(), &tool_defs, model)
                .await
                .map_err(|e| {
                    let error_msg = format!("LLM 调用失败: {}", e);
                    eprintln!("[Agent] {}", error_msg);
                    let _ = channel.send(StreamChunk::Error { message: error_msg.clone() });
                    error_msg
                })?;

            match result {
                ChatSyncResult::Content(text) => {
                    // 批量发送文本，每 15 个字符为一组，2ms 间隔保持视觉打字效果
                    let chars: Vec<char> = text.chars().collect();
                    for chunk in chars.chunks(15) {
                        let _ = channel.send(StreamChunk::TextDelta {
                            delta: chunk.iter().collect::<String>(),
                        });
                        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                    }
                    let _ = channel.send(StreamChunk::Done { finish_reason: Some("stop".into()) });

                    // 保存记忆
                    if !user_task.is_empty() {
                        let memory = Memory {
                            id: None,
                            task: user_task.to_string(),
                            solution: text.clone(),
                            tools_used: serde_json::to_string(&tools_used).unwrap_or_default(),
                            success: true,
                            timestamp: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() as i64,
                            failure_reason: None,
                            fix_applied: None,
                        };
                        let _ = global_memory().save(&memory);
                    }

                    return Ok(AgentResult {
                        success: true,
                        final_answer: text,
                        iterations: iteration + 1,
                        steps: self.steps.clone(),
                    });
                }
                ChatSyncResult::ToolCalls { calls, reasoning } => {
                    // 发送思考内容（如果有），前端以 ThinkingBubble 展示
                    if let Some(ref r) = reasoning {
                        if !r.is_empty() {
                            for ch in r.chars() {
                                let _ = channel.send(StreamChunk::ThinkingDelta { delta: ch.to_string() });
                            }
                        }
                    }
                    for tc in &calls {
                        let _ = channel.send(StreamChunk::ToolCallStart { id: tc.id.clone(), name: tc.name.clone() });
                        let _ = channel.send(StreamChunk::ToolCallDelta { id: tc.id.clone(), arguments_delta: tc.arguments.to_string() });
                        let _ = channel.send(StreamChunk::ToolCallEnd { id: tc.id.clone() });
                    }
                    let _ = channel.send(StreamChunk::ThinkingDone {});

                    self.context.push(UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: vec![],
                        tool_calls: Some(calls.clone()),
                        tool_call_id: None,
                        reasoning_content: reasoning.clone(),
                    });

                    let mut actions = Vec::new();
                    for tc in &calls {
                        tools_used.push(tc.name.clone());
                        let result = if tc.name.starts_with("mcp_") {
                            execute_mcp_tool(tc).await
                        } else {
                            self.tool_registry.execute(&tc.name, &tc.arguments, &self.config.work_dir).await
                        };

                        let _ = channel.send(StreamChunk::ToolResult {
                            call_id: tc.id.clone(),
                            name: tc.name.clone(),
                            success: result.success,
                            content: result.content.clone(),
                        });

                        actions.push(AgentAction {
                            tool_name: tc.name.clone(),
                            arguments: tc.arguments.clone(),
                            result: result.content.clone(),
                            success: result.success,
                        });

                        self.context.push(UnifiedMessage {
                            role: MessageRole::Tool,
                            content: vec![ContentBlock::Text { text: result.content.clone() }],
                            tool_calls: None,
                            tool_call_id: Some(tc.id.clone()),
                            reasoning_content: None,
                        });
                    }

                    let observation = actions.iter().map(|a| format!("{}: {}", a.tool_name, a.result)).collect::<Vec<_>>().join("\n");
                    self.steps.push(AgentStep {
                        iteration,
                        thought: reasoning.unwrap_or_else(|| "执行工具调用".into()),
                        action: actions.into_iter().next(),
                        observation: Some(observation),
                    });
                }
            }
        }

        let error_msg = format!("达到最大迭代次数 ({})", self.config.max_iterations);
        let _ = channel.send(StreamChunk::Error { message: error_msg.clone() });
        let _ = channel.send(StreamChunk::Done { finish_reason: Some("max_iterations".into()) });
        Err(error_msg)
    }

    /// 执行分解后的任务
    async fn run_with_decomposed_tasks<T: crate::commands::chat::StreamSender + Clone>(
        &mut self,
        provider: &dyn LlmProvider,
        mut tasks: Vec<Task>,
        model: &str,
        channel: T,
        user_task: &str,
    ) -> Result<AgentResult, String> {
        let mut all_results = Vec::new();

        while tasks.iter().any(|t| t.status != TaskStatus::Completed && t.status != TaskStatus::Failed) {
            let ready_ids = TaskDecomposer::get_ready_tasks(&tasks);
            if ready_ids.is_empty() {
                break;
            }

            // 并行执行可执行任务
            for task_id in ready_ids {
                if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                    task.status = TaskStatus::Running;

                    // 为子任务创建新上下文
                    let sub_context = vec![UnifiedMessage {
                        role: MessageRole::User,
                        content: vec![ContentBlock::Text { text: task.description.clone() }],
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    }];

                    // 执行子任务（单次迭代）
                    let task_desc = task.description.chars().take(40).collect::<String>();
                    let _ = channel.send(StreamChunk::StatusDelta {
                        message: format!("执行子任务: {}", task_desc),
                    });

                    let tool_defs = self.tool_registry.get_all_definitions();
                    match provider.chat_sync_with_tools(sub_context, &tool_defs, model).await {
                        Ok(ChatSyncResult::ToolCalls { calls, .. }) => {
                            for tc in &calls {
                                // 发送结构化工具调用事件
                                let _ = channel.send(StreamChunk::ToolCallStart {
                                    id: tc.id.clone(),
                                    name: tc.name.clone(),
                                });
                                let _ = channel.send(StreamChunk::ToolCallDelta {
                                    id: tc.id.clone(),
                                    arguments_delta: tc.arguments.to_string(),
                                });
                                let _ = channel.send(StreamChunk::ToolCallEnd {
                                    id: tc.id.clone(),
                                });

                                let result = if tc.name.starts_with("mcp_") {
                                    execute_mcp_tool(tc).await
                                } else {
                                    self.tool_registry.execute(&tc.name, &tc.arguments, &self.config.work_dir).await
                                };

                                let _ = channel.send(StreamChunk::ToolResult {
                                    call_id: tc.id.clone(),
                                    name: tc.name.clone(),
                                    success: result.success,
                                    content: result.content.clone(),
                                });

                                task.result = Some(result.content.clone());
                                task.status = if result.success { TaskStatus::Completed } else { TaskStatus::Failed };
                                all_results.push(result.content);
                            }
                        }
                        Ok(ChatSyncResult::Content(text)) => {
                            task.result = Some(text.clone());
                            task.status = TaskStatus::Completed;
                            all_results.push(text);
                        }
                        Err(e) => {
                            task.status = TaskStatus::Failed;
                            task.result = Some(format!("失败: {}", e));
                        }
                    }
                }
            }
        }

        // 将所有子任务结果汇总，交由 LLM 生成最终回答
        let subtask_summary: String = tasks
            .iter()
            .enumerate()
            .map(|(i, t)| {
                format!(
                    "子任务 {}: {}\n状态: {}\n结果: {}",
                    i + 1,
                    t.description,
                    match t.status {
                        TaskStatus::Completed => "完成",
                        TaskStatus::Failed => "失败",
                        _ => "未完成",
                    },
                    t.result.as_deref().unwrap_or("无")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let summary_prompt = format!(
            "以下是多个子任务的执行结果。请根据这些结果，用自然语言回答用户的原始需求。\n\n\
             原始需求: {}\n\n\
             子任务执行结果:\n{}\n\n\
             请综合以上信息，给出一个完整、清晰的回答。直接回答，不要提及\"子任务\"或执行过程。",
            user_task, subtask_summary
        );

        let summary_messages = vec![UnifiedMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: summary_prompt,
            }],
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        }];

        let _ = channel.send(StreamChunk::StatusDelta {
            message: "汇总子任务结果中".into(),
        });

        match provider.chat_sync(summary_messages, model).await {
            Ok(final_answer) => {
                let chars: Vec<char> = final_answer.chars().collect();
                for chunk in chars.chunks(15) {
                    let _ = channel.send(StreamChunk::TextDelta {
                        delta: chunk.iter().collect::<String>(),
                    });
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                let _ = channel.send(StreamChunk::Done {
                    finish_reason: Some("stop".into()),
                });

                Ok(AgentResult {
                    success: true,
                    final_answer,
                    iterations: tasks.len(),
                    steps: self.steps.clone(),
                })
            }
            Err(e) => {
                // LLM 摘要失败，回退到机械拼接
                let fallback = format!(
                    "任务执行完成，以下是各子任务结果：\n\n{}",
                    subtask_summary
                );
                let chars: Vec<char> = fallback.chars().collect();
                for chunk in chars.chunks(15) {
                    let _ = channel.send(StreamChunk::TextDelta {
                        delta: chunk.iter().collect::<String>(),
                    });
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
                let _ = channel.send(StreamChunk::Error {
                    message: format!("摘要生成失败: {}", e),
                });

                Ok(AgentResult {
                    success: false,
                    final_answer: fallback,
                    iterations: tasks.len(),
                    steps: self.steps.clone(),
                })
            }
        }
    }
}

/// 执行 MCP 工具
async fn execute_mcp_tool(tc: &crate::providers::types::ToolCall) -> tools::ToolResult {
    let mcp = global_mcp();
    let servers = mcp.list_servers().await;

    for server in &servers {
        if !server.enabled {
            continue;
        }
        match mcp
            .execute_mcp_tool(&server.id, &tc.name, &tc.arguments)
            .await
        {
            Ok(content) => {
                return tools::ToolResult {
                    success: true,
                    content,
                };
            }
            Err(_) => continue,
        }
    }

    tools::ToolResult {
        success: false,
        content: format!("MCP 工具 {} 执行失败：未找到可用的 MCP 服务器", tc.name),
    }
}

/// 判断任务是否复杂（需要分解）
/// 只有明确包含多步骤关键词时才触发分解，避免简单任务被误判
pub fn is_complex_task(task: &str) -> bool {
    let keywords = [
        "并且", "然后", "接着", "同时", "以及",
        "首先", "第一步", "第二步",
        "先...再", "先", "再",
        "做完", "之后", "然后再",
        "多个文件", "批量", "所有文件",
        "每个", "遍历",
    ];
    keywords.iter().any(|k| task.contains(k))
}
