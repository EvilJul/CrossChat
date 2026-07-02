pub mod react;
pub mod task_decomposer;
pub mod tool_registry;
pub mod compactor;

use crate::mcp::global_mcp;
use crate::memory::{global_memory, Memory};
use crate::providers::types::{ChatSyncResult, ContentBlock, MessageRole, StreamChunk, UnifiedMessage};
use crate::providers::LlmProvider;
use crate::tools;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
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

#[derive(Clone)]
pub struct NoOpStreamSender;
impl crate::commands::chat::StreamSender for NoOpStreamSender {
    fn send(&self, _chunk: StreamChunk) -> Result<(), String> {
        Ok(())
    }
}

/// 核心 Agent 结构
pub struct Agent {
    config: AgentConfig,
    tool_registry: ToolRegistry,
    context: Vec<UnifiedMessage>,
    steps: Vec<AgentStep>,
    pub depth: usize,
    pub spawn_count: usize,
    pub total_child_runs: usize,
    pub prompt_tokens: usize,
    pub completion_tokens: usize,
    pub allowed_tools: Option<Vec<String>>,
}

impl Agent {
    pub fn new(config: AgentConfig, tool_registry: ToolRegistry) -> Self {
        Self {
            config,
            tool_registry,
            context: Vec::new(),
            steps: Vec::new(),
            depth: 0,
            spawn_count: 0,
            total_child_runs: 0,
            prompt_tokens: 0,
            completion_tokens: 0,
            allowed_tools: None,
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
        self.initialize_skills_and_gating();

        for iteration in 0..self.config.max_iterations {
            let tool_defs = self.tool_registry.get_all_definitions();

            // 累计估算 prompt tokens
            let prompt_toks = crate::agent::compactor::ContextCompactor::estimate_tokens(&self.context);
            self.prompt_tokens += prompt_toks;

            // 调用 LLM 思考并决定行动
            let result = provider
                .chat_sync_with_tools(self.context.clone(), &tool_defs, model)
                .await
                .map_err(|e| format!("LLM 调用失败: {}", e))?;

            match result {
                ChatSyncResult::Content(text) => {
                    // 累计估算 completion tokens
                    let response_msg = UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: vec![ContentBlock::Text { text: text.clone() }],
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    };
                    self.completion_tokens += crate::agent::compactor::ContextCompactor::estimate_tokens(&[response_msg]);

                    // LLM 给出最终答案
                    return Ok(AgentResult {
                        success: true,
                        final_answer: text,
                        iterations: iteration + 1,
                        steps: self.steps.clone(),
                    });
                }
                ChatSyncResult::ToolCalls { calls, reasoning } => {
                    // 累计估算 completion tokens (来自思考和工具调用)
                    let response_msg = UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: vec![],
                        tool_calls: Some(calls.clone()),
                        tool_call_id: None,
                        reasoning_content: reasoning.clone(),
                    };
                    self.completion_tokens += crate::agent::compactor::ContextCompactor::estimate_tokens(&[response_msg]);

                    // 记录思考过程
                    let thought = reasoning.clone().unwrap_or_else(|| "执行工具调用".into());

                    // 执行所有工具调用
                    let mut actions = Vec::new();
                    for tc in &calls {
                        let result = if !self.is_tool_allowed(&tc.name) {
                            tools::ToolResult {
                                success: false,
                                content: format!("Security Error: Tool '{}' is blocked. This turn is gated by the active skills and only allowed tools can be executed.", tc.name),
                            }
                        } else if tc.name == "delegate_task" {
                            self.spawn_count += 1;
                            self.handle_delegate_task(tc, provider, model, NoOpStreamSender).await
                        } else if tc.name == "mcp_search" {
                            self.handle_mcp_search(tc).await
                        } else if tc.name == "mcp_describe" {
                            self.handle_mcp_describe(tc).await
                        } else if tc.name == "mcp_call" {
                            self.handle_mcp_call(tc).await
                        } else if tc.name == "mcp_refresh_catalog" {
                            self.handle_mcp_refresh_catalog().await
                        } else if tc.name.starts_with("mcp_") {
                            execute_mcp_tool(tc, &self.config.work_dir).await
                        } else {
                            self.tool_registry
                                .execute(&tc.name, &tc.arguments, &self.config.work_dir)
                                .await
                        };

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
        self.initialize_skills_and_gating();

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

            // 累计估算 prompt tokens
            let prompt_toks = crate::agent::compactor::ContextCompactor::estimate_tokens(&self.context);
            self.prompt_tokens += prompt_toks;

            // 发送进度提示，避免前端长时间无反馈
            let _ = channel.send(StreamChunk::StatusDelta {
                message: format!("Thinking (iteration {})", iteration + 1),
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
                    // 累计估算 completion tokens
                    let response_msg = UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: vec![ContentBlock::Text { text: text.clone() }],
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    };
                    self.completion_tokens += crate::agent::compactor::ContextCompactor::estimate_tokens(&[response_msg]);

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
                    // 累计估算 completion tokens (来自思考和工具调用)
                    let response_msg = UnifiedMessage {
                        role: MessageRole::Assistant,
                        content: vec![],
                        tool_calls: Some(calls.clone()),
                        tool_call_id: None,
                        reasoning_content: reasoning.clone(),
                    };
                    self.completion_tokens += crate::agent::compactor::ContextCompactor::estimate_tokens(&[response_msg]);

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
                        let result = if !self.is_tool_allowed(&tc.name) {
                            tools::ToolResult {
                                success: false,
                                content: format!("Security Error: Tool '{}' is blocked. This turn is gated by the active skills and only allowed tools can be executed.", tc.name),
                            }
                        } else if tc.name == "delegate_task" {
                            self.spawn_count += 1;
                            self.handle_delegate_task(tc, provider, model, channel.clone()).await
                        } else if tc.name == "mcp_search" {
                            self.handle_mcp_search(tc).await
                        } else if tc.name == "mcp_describe" {
                            self.handle_mcp_describe(tc).await
                        } else if tc.name == "mcp_call" {
                            self.handle_mcp_call(tc).await
                        } else if tc.name == "mcp_refresh_catalog" {
                            self.handle_mcp_refresh_catalog().await
                        } else if tc.name.starts_with("mcp_") {
                            execute_mcp_tool(tc, &self.config.work_dir).await
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

                                let result = if !self.is_tool_allowed(&tc.name) {
                                    tools::ToolResult {
                                        success: false,
                                        content: format!("Security Error: Tool '{}' is blocked. This turn is gated by the active skills and only allowed tools can be executed.", tc.name),
                                    }
                                } else if tc.name.starts_with("mcp_") {
                                    execute_mcp_tool(tc, &self.config.work_dir).await
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

    /// 处理子智能体任务委派 (delegate_task)
    pub fn handle_delegate_task<'a, T: crate::commands::chat::StreamSender + Clone + Send + 'a>(
        &'a mut self,
        tc: &'a crate::providers::types::ToolCall,
        provider: &'a dyn LlmProvider,
        parent_model: &'a str,
        channel: T,
    ) -> futures_util::future::BoxFuture<'a, tools::ToolResult> {
        use futures_util::future::FutureExt;
        async move {
            // 1. 预算限制校验
            if self.total_child_runs >= 10 {
                return tools::ToolResult {
                    success: false,
                    content: "Error: Max delegation budget (10 runs) reached inside the thread tree.".into(),
                };
            }

            // 2. 解析参数
            let prompt = match tc.arguments["prompt"].as_str() {
                Some(p) => p.to_string(),
                None => {
                    return tools::ToolResult {
                        success: false,
                        content: "Error: Missing required argument 'prompt'.".into(),
                    };
                }
            };

            let label = tc.arguments["label"].as_str().unwrap_or("subtask");
            let workspace = tc.arguments["workspace"].as_str().unwrap_or(&self.config.work_dir);
            let model = tc.arguments["model"].as_str().unwrap_or(parent_model);

            let _ = channel.send(StreamChunk::StatusDelta {
                message: format!("[Delegation] Spawning sub-agent for '{}'", label),
            });

            // 3. 创建子智能体实例并隔离状态
            let mut child_config = self.config.clone();
            child_config.work_dir = workspace.to_string();
            child_config.enable_task_decomposition = false; // 子智能体不执行并行任务拆分

            let mut child_registry = ToolRegistry::new();
            // 注册同样的内置工具
            for tool in tools::get_all_tool_definitions() {
                child_registry.register(tool);
            }

            let mut child_agent = Agent::new(child_config, child_registry);
            child_agent.depth = self.depth + 1;
            child_agent.total_child_runs = self.total_child_runs + 1;
            child_agent.spawn_count = 0;

            // 构建内存隔离的初始消息上下文
            let child_initial_context = vec![
                crate::providers::types::UnifiedMessage {
                    role: crate::providers::types::MessageRole::User,
                    content: vec![crate::providers::types::ContentBlock::Text {
                        text: format!(
                            "[Task Delegation - 子沙盒运行]\n\
                             你是一个被委派的子智能体。你的目标是完成以下具体任务:\n\n\
                             任务指令: {}\n\n\
                             重要: 请在当前工作区独立完成该任务，并在结束时输出清晰的最终回答。",
                            prompt
                        ),
                    }],
                    tool_calls: None,
                    tool_call_id: None,
                    reasoning_content: None,
                }
            ];

            // 4. 阻塞运行子智能体
            let start_time = std::time::Instant::now();
            let child_result = child_agent.run_with_stream(
                provider,
                child_initial_context,
                model,
                channel.clone()
            ).await;

            let duration_ms = start_time.elapsed().as_millis();

            // 5. 用量累计 Roll-up
            let child_prompt = child_agent.prompt_tokens;
            let child_completion = child_agent.completion_tokens;

            self.prompt_tokens += child_prompt;
            self.completion_tokens += child_completion;

            // 更新父智能体总计运行数
            self.total_child_runs = child_agent.total_child_runs;

            // 6. 处理执行结果并警告多重调用
            let warning_suffix = if self.spawn_count > 1 {
                format!(
                    "\n\n[Warning: Thread tree contains multiple child agent runs. \
                     Current parent spawn_count: {}. Please avoid redundant recursive calls.]",
                    self.spawn_count
                )
            } else {
                "".to_string()
            };

            match child_result {
                Ok(res) => {
                    let _ = channel.send(StreamChunk::StatusDelta {
                        message: format!(
                            "[Delegation] Sub-agent '{}' completed successfully in {}ms (used estimated {} prompt, {} completion tokens)",
                            label, duration_ms, child_prompt, child_completion
                        ),
                    });
                    tools::ToolResult {
                        success: true,
                        content: format!(
                            "Sub-agent completed target task successfully.\n\n\
                             Final Summary:\n{}\n\n\
                             Execution Time: {}ms{}",
                            res.final_answer, duration_ms, warning_suffix
                        ),
                    }
                }
                Err(err) => {
                    let _ = channel.send(StreamChunk::StatusDelta {
                        message: format!("[Delegation] Sub-agent '{}' failed: {}", label, err),
                    });
                    tools::ToolResult {
                        success: false,
                        content: format!(
                            "Sub-agent execution encountered an error.\n\n\
                             Error Details: {}{}",
                            err, warning_suffix
                        ),
                    }
                }
            }
        }.boxed()
    }

    /// 处理 MCP 工具库检索 (BM25 模糊匹配算法)
    pub async fn handle_mcp_search(&self, tc: &crate::providers::types::ToolCall) -> tools::ToolResult {
        let query = tc.arguments["query"].as_str().unwrap_or("").to_lowercase();
        if query.is_empty() {
            return tools::ToolResult {
                success: false,
                content: "Error: Query cannot be empty.".into(),
            };
        }

        let mut matched = Vec::new();
        for (name, def) in self.tool_registry.get_tools() {
            if !name.starts_with("mcp_") {
                continue;
            }

            let mut score = 0.0;
            let name_lower = name.to_lowercase();
            let desc_lower = def.description.to_lowercase();
            let params_str = def.parameters.to_string().to_lowercase();

            // 扩展查询词同义词权重
            let mut expanded_queries = vec![query.clone()];
            if query.contains("find") || query.contains("search") || query.contains("查找") || query.contains("搜索") {
                expanded_queries.push("find".to_string());
                expanded_queries.push("search".to_string());
                expanded_queries.push("get".to_string());
            }
            if query.contains("update") || query.contains("modify") || query.contains("edit") || query.contains("修改") || query.contains("编辑") {
                expanded_queries.push("update".to_string());
                expanded_queries.push("modify".to_string());
                expanded_queries.push("edit".to_string());
                expanded_queries.push("write".to_string());
            }

            for q in &expanded_queries {
                // 工具名称匹配：x5.0
                if name_lower.contains(q) {
                    score += 5.0;
                }
                // 工具描述匹配：x1.0
                if desc_lower.contains(q) {
                    score += 1.0;
                }
                // 工具入参匹配：x2.0
                if params_str.contains(q) {
                    score += 2.0;
                }
            }

            if score >= 0.15 {
                matched.push((name.clone(), def.description.clone(), score));
            }
        }

        matched.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        let top_matches = matched.iter().take(5);

        let mut result_content = String::from("Found matching MCP tools:\n\n");
        for (name, desc, score) in top_matches {
            let _ = writeln!(result_content, "- **{}**: {} (Score: {:.2})", name, desc, score);
        }

        if matched.is_empty() {
            result_content = "No matching MCP tools found. Please try a different query.".into();
        }

        tools::ToolResult {
            success: true,
            content: result_content,
        }
    }

    /// 获取特定 MCP 工具的参数细节
    pub async fn handle_mcp_describe(&self, tc: &crate::providers::types::ToolCall) -> tools::ToolResult {
        let tool_name = tc.arguments["tool_name"].as_str().unwrap_or("");
        if let Some(def) = self.tool_registry.get_tools().get(tool_name) {
            tools::ToolResult {
                success: true,
                content: format!(
                    "Tool: {}\nDescription: {}\nParameters Schema:\n{}",
                    tool_name,
                    def.description,
                    serde_json::to_string_pretty(&def.parameters).unwrap_or_default()
                ),
            }
        } else {
            tools::ToolResult {
                success: false,
                content: format!("Error: Tool '{}' not found in registry.", tool_name),
            }
        }
    }

    /// 调用 MCP 工具
    pub async fn handle_mcp_call(&self, tc: &crate::providers::types::ToolCall) -> tools::ToolResult {
        let tool_name = tc.arguments["tool_name"].as_str().unwrap_or("");
        let args = tc.arguments.get("arguments").cloned().unwrap_or(serde_json::json!({}));

        let mut mock_call = tc.clone();
        mock_call.name = tool_name.to_string();
        mock_call.arguments = args;

        execute_mcp_tool(&mock_call, &self.config.work_dir).await
    }

    /// 重新加载 MCP 工具目录
    pub async fn handle_mcp_refresh_catalog(&self) -> tools::ToolResult {
        let mcp = global_mcp();
        let _ = mcp.get_all_tools("").await;
        tools::ToolResult {
            success: true,
            content: "MCP tool catalog refreshed successfully.".into(),
        }
    }

    pub fn is_tool_allowed(&self, name: &str) -> bool {
        if let Some(ref allowed) = self.allowed_tools {
            // Check exact match
            if allowed.contains(&name.to_string()) {
                return true;
            }
            // Check without "mcp_" prefix
            if name.starts_with("mcp_") {
                let without_prefix = &name[4..];
                if allowed.contains(&without_prefix.to_string()) {
                    return true;
                }
            }
            // Meta-tools used for MCP discovery and delegation are always allowed
            if name == "mcp_search" || name == "mcp_describe" || name == "mcp_call" || name == "mcp_refresh_catalog" || name == "delegate_task" {
                return true;
            }
            false
        } else {
            true
        }
    }

    pub fn initialize_skills_and_gating(&mut self) {
        let user_task = self.context.iter()
            .find(|m| matches!(m.role, MessageRole::User))
            .and_then(|m| m.content.iter().find_map(|c| {
                if let ContentBlock::Text { text } = c { Some(text.clone()) } else { None }
            }))
            .unwrap_or_default();

        if !user_task.is_empty() {
            let mut file_paths = Vec::new();
            for word in user_task.split(|c: char| c.is_whitespace() || c == '`' || c == '\'' || c == '"') {
                let trimmed = word.trim_matches(|c: char| c.is_ascii_punctuation() && c != '.' && c != '/' && c != '\\' && c != '_');
                if trimmed.contains('.') && trimmed.len() > 2 {
                    file_paths.push(trimmed.to_string());
                }
            }

            let (skill_ctx, allowed_tools) = crate::skills::global_skills().get_activated_skills_context(&user_task, &file_paths);
            if let Some(ctx) = skill_ctx {
                let already_has = self.context.iter().any(|m| {
                    if matches!(m.role, MessageRole::User) {
                        m.content.iter().any(|c| {
                            if let ContentBlock::Text { text } = c {
                                text.contains("[已激活的 Skills — 已针对当前任务自动挂载扩展能力]")
                            } else {
                                false
                            }
                        })
                    } else {
                        false
                    }
                });
                if !already_has {
                    self.context.insert(0, UnifiedMessage {
                        role: MessageRole::User,
                        content: vec![ContentBlock::Text { text: ctx }],
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
            }
            if self.allowed_tools.is_none() {
                self.allowed_tools = allowed_tools;
            }
        }
    }
}

/// 执行 MCP 工具
async fn execute_mcp_tool(tc: &crate::providers::types::ToolCall, work_dir: &str) -> tools::ToolResult {
    let mcp = global_mcp();
    let servers = mcp.list_servers().await;
    
    let mut errors = Vec::new();
    let mut enabled_count = 0;

    for server in &servers {
        if !server.enabled {
            continue;
        }

        // 校验工作区沙盒可信作用域
        if !crate::mcp::is_mcp_server_trusted(server, work_dir) {
            errors.push(format!("• 服务器 '{}' 未被授权在工作区 '{}' 运行 (Trust Scope)", server.name, work_dir));
            continue;
        }
        
        enabled_count += 1;
        
        match mcp.execute_mcp_tool(&server.id, &tc.name, &tc.arguments).await {
            Ok(content) => {
                eprintln!("[MCP] 工具 {} 在服务器 '{}' 上执行成功", tc.name, server.name);
                return tools::ToolResult {
                    success: true,
                    content,
                };
            }
            Err(e) => {
                eprintln!("[MCP] 工具 {} 在服务器 '{}' 上执行失败: {}", tc.name, server.name, e);
                errors.push(format!("• 服务器 '{}' ({}): {}", server.name, server.command, e));
            }
        }
    }

    // 构建详细的错误信息
    let error_msg = if enabled_count == 0 {
        format!(
            "❌ MCP 工具 {} 执行失败\n\n\
             原因：没有启用的 MCP 服务器\n\n\
             💡 解决方法：\n\
             1. 在设置中添加并启用 MCP 服务器\n\
             2. 确保服务器配置正确\n\
             3. 检查命令和参数是否正确",
            tc.name
        )
    } else if errors.is_empty() {
        format!(
            "❌ MCP 工具 {} 执行失败\n\n\
             原因：未知错误（没有启用的服务器返回结果）",
            tc.name
        )
    } else {
        format!(
            "❌ MCP 工具 {} 执行失败\n\n\
             尝试了 {} 个服务器，全部失败：\n\n{}\n\n\
             💡 可能的原因：\n\
             1. 工具名称不匹配（工具可能不存在于这些服务器中）\n\
             2. 服务器启动失败或超时\n\
             3. 参数格式错误\n\
             4. 网络连接问题\n\n\
             🔧 建议：\n\
             1. 检查工具名称是否正确\n\
             2. 在设置中测试 MCP 服务器连接\n\
             3. 查看服务器日志了解详细错误",
            tc.name,
            enabled_count,
            errors.join("\n")
        )
    };

    tools::ToolResult {
        success: false,
        content: error_msg,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_tool_gating() {
        let config = AgentConfig {
            max_iterations: 5,
            work_dir: "/tmp".to_string(),
            enable_self_healing: false,
            enable_task_decomposition: false,
        };
        let registry = ToolRegistry::new();
        let mut agent = Agent::new(config, registry);

        // Initially no allowed_tools restrictions
        assert!(agent.is_tool_allowed("read_file"));
        assert!(agent.is_tool_allowed("run_command"));

        // With allowed_tools restrictions
        agent.allowed_tools = Some(vec!["read_file".to_string(), "write_file".to_string()]);
        assert!(agent.is_tool_allowed("read_file"));
        assert!(!agent.is_tool_allowed("run_command"));

        // MCP tools and prefixes
        assert!(agent.is_tool_allowed("mcp_read_file"));
        assert!(!agent.is_tool_allowed("mcp_run_command"));

        // Meta-tools should always be allowed
        assert!(agent.is_tool_allowed("mcp_search"));
        assert!(agent.is_tool_allowed("delegate_task"));
    }
}
