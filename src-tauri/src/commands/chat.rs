use crate::commands::agent_cmd::read_agent_config;
use crate::mcp::global_mcp;
use crate::providers::openai_compat::{ChatSyncResult, OpenAICompatProvider};
use crate::providers::types::{ContentBlock, MessageRole, StreamChunk, UnifiedMessage};
use crate::providers::LlmProvider;
use crate::tools::{self};
use serde::Deserialize;
use tauri::ipc::Channel;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub provider_id: String,
    pub model: String,
    pub messages: Vec<UnifiedMessage>,
    #[serde(default)]
    pub api_base: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub provider_type: Option<String>,
    #[serde(default)]
    pub work_dir: Option<String>,
}

/// 流式聊天命令（含工具调用循环）
#[tauri::command]
pub async fn stream_chat(
    request: ChatRequest,
    channel: Channel<StreamChunk>,
) -> Result<(), String> {
    let has_creds = request.api_key.is_some() && request.api_base.is_some();

    if has_creds {
        let api_key = request.api_key.unwrap();
        let api_base = request.api_base.unwrap();
        let provider_type = request.provider_type.as_deref().unwrap_or("openai-compat");
        let work_dir = request.work_dir.unwrap_or_default();

        match provider_type {
            "openai-compat" => {
                let provider = OpenAICompatProvider::new(api_base, api_key);
                run_tool_loop(&provider, request.messages, &request.model, channel, &work_dir)
                    .await
                    .map_err(|e| format!("Provider 错误: {}", e))?;
            }
            _ => {
                let _ = channel.send(StreamChunk::Error {
                    message: format!("不支持的 Provider 类型: {}", provider_type),
                });
                return Err(format!("不支持的 Provider 类型: {}", provider_type));
            }
        }
    } else {
        simulate_demo_chat(&request, channel).await;
    }

    Ok(())
}

/// 估算消息的 token 数量（粗略：中文 ~1.5 char/token，英文 ~3.5 char/token）
fn estimate_tokens(messages: &[UnifiedMessage]) -> usize {
    let mut total = 0usize;
    for msg in messages {
        for block in &msg.content {
            let ContentBlock::Text { text } = block;
            // 混合估算：中文按 1.5，英文按 3.5
            let chinese_chars = text.chars().filter(|c| c > &'\u{2FFF}').count();
            let other_chars = text.len() - chinese_chars;
            total += (chinese_chars as f64 / 1.5 + other_chars as f64 / 3.5).ceil() as usize;
        }
        // 工具调用参数也计入
        if let Some(ref tcs) = msg.tool_calls {
            for tc in tcs {
                let args = tc.arguments.to_string();
                total += (args.len() as f64 / 3.5).ceil() as usize;
            }
        }
    }
    total
}

/// 自动上下文压缩（类似 Claude Code 机制）
/// 当消息超过 token 阈值时，用 LLM 自动摘要前 60% 的消息，
/// 将摘要注入为 system 消息，保留最新 40% 的消息。
async fn auto_compress_context(
    provider: &OpenAICompatProvider,
    messages: Vec<UnifiedMessage>,
    model: &str,
    channel: Channel<StreamChunk>,
) -> Vec<UnifiedMessage> {
    const COMPRESS_TOKEN_THRESHOLD: usize = 100_000; // 100K tokens 时触发压缩（200K 窗口的 50%）
    const KEEP_RECENT_RATIO: f64 = 0.4; // 保留最近 40% 的消息

    let estimated = estimate_tokens(&messages);
    if estimated < COMPRESS_TOKEN_THRESHOLD || messages.len() < 10 {
        return messages; // 无需压缩
    }

    let split_point = (messages.len() as f64 * (1.0 - KEEP_RECENT_RATIO)).ceil() as usize;
    if split_point < 3 {
        return messages; // 消息太少，不压缩
    }

    let (old_messages, recent_messages) = messages.split_at(split_point);
    let old_messages = old_messages.to_vec();
    let mut recent_messages = recent_messages.to_vec();

    // 通知前端正在进行压缩
    let _ = channel.send(StreamChunk::TextDelta {
        delta: "\n\n> *上下文压缩中... 正在摘要早期对话*\n\n".into(),
    });

    // 构建摘要请求
    let summary_prompt = format!(
        "请用简洁的要点形式总结以下对话历史。保留关键信息：用户的需求、已完成的文件操作、代码修改、\
         做出的决策、当前任务的进度。用中文回复，不超过 500 字。\n\n---\n{}",
        old_messages
            .iter()
            .filter_map(|m| {
                let text: String = m
                    .content
                    .iter()
                    .filter_map(|b| {
                        let ContentBlock::Text { text } = b;
                        Some(text.as_str())
                    })
                    .collect();
                if text.trim().is_empty() {
                    return None;
                }
                let role_str = match m.role {
                    MessageRole::User => "用户",
                    MessageRole::Assistant => "助手",
                    MessageRole::Tool => "工具结果",
                };
                Some(format!("[{}]: {}", role_str, text))
            })
            .collect::<Vec<_>>()
            .join("\n")
    );

    // 用 LLM 生成摘要
    match provider
        .chat_sync(
            vec![UnifiedMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text {
                    text: summary_prompt,
                }],
                tool_calls: None,
                tool_call_id: None,
                    reasoning_content: None,
            }],
            model,
        )
        .await
    {
        Ok(summary) => {
            // 将摘要作为 system 消息插入
            let compressed_msg = UnifiedMessage {
                role: MessageRole::User, // 用 user 角色发送系统级摘要
                content: vec![ContentBlock::Text {
                    text: format!("[上下文摘要 — 之前对话的要点]\n{}", summary.trim()),
                }],
                tool_calls: None,
                tool_call_id: None,
                    reasoning_content: None,
            };

            let mut compressed = vec![compressed_msg];
            compressed.append(&mut recent_messages);

            let new_estimated = estimate_tokens(&compressed);
            let _ = channel.send(StreamChunk::TextDelta {
                delta: format!(
                    "> *压缩完成: {} → {} tokens (压缩率 {:.0}%)*\n\n",
                    estimated,
                    new_estimated,
                    (1.0 - new_estimated as f64 / estimated as f64) * 100.0
                )
                .into(),
            });

            compressed
        }
        Err(_) => {
            // 摘要失败，回退到简单截断
            let _ = channel.send(StreamChunk::TextDelta {
                delta: "> *上下文压缩失败，使用截断模式*\n\n".into(),
            });
            recent_messages
        }
    }
}

/// 工具调用循环：LLM → 检测 tool_calls → 执行 → 结果回传 → 继续
async fn run_tool_loop(
    provider: &OpenAICompatProvider,
    mut messages: Vec<UnifiedMessage>,
    model: &str,
    channel: Channel<StreamChunk>,
    work_dir: &str,
) -> Result<(), crate::providers::ProviderError> {
    // 注入 AGENT.md 约束（如果存在）
    let agent_config = read_agent_config(Some(work_dir.to_string())).unwrap_or_default();
    if agent_config.found && !agent_config.merged.trim().is_empty() {
        let system_msg = UnifiedMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: format!(
                    "[系统指令 — 遵循以下规则约束你的行为]\n\n{}\n\n---\n请始终遵循以上规则。",
                    agent_config.merged.trim()
                ),
            }],
            tool_calls: None,
            tool_call_id: None,
                    reasoning_content: None,
        };
        messages.insert(0, system_msg);
    }

    // 上下文压缩：超阈值时自动摘要旧消息（类似 Claude Code 机制）
    messages = auto_compress_context(provider, messages, model, channel.clone()).await;

    // 合并内置工具 + MCP 工具
    let mut tool_definitions = tools::get_all_tool_definitions();
    let mcp_tools = global_mcp().get_all_tools().await;
    tool_definitions.extend(mcp_tools);
    let max_iterations = 5;

    for _iteration in 0..max_iterations {
        // 用非流式调用检测是否需要工具调用
        let result = provider
            .chat_sync_with_tools(messages.clone(), &tool_definitions, model)
            .await?;

        match result {
            ChatSyncResult::Content(text) => {
                for ch in text.chars() {
                    let _ = channel.send(StreamChunk::TextDelta { delta: ch.to_string() });
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
                let _ = channel.send(StreamChunk::Done { finish_reason: Some("stop".into()) });
                return Ok(());
            }
            ChatSyncResult::ToolCalls { calls: tool_calls, reasoning } => {
                for tc in &tool_calls {
                    let _ = channel.send(StreamChunk::ToolCallStart { id: tc.id.clone(), name: tc.name.clone() });
                    let _ = channel.send(StreamChunk::ToolCallDelta { id: tc.id.clone(), arguments_delta: tc.arguments.to_string() });
                    let _ = channel.send(StreamChunk::ToolCallEnd { id: tc.id.clone() });
                }

                // 构建 assistant 消息（含 tool_calls + reasoning_content 回传）
                let assistant_msg = UnifiedMessage {
                    role: MessageRole::Assistant,
                    content: vec![],
                    tool_calls: Some(tool_calls.clone()),
                    tool_call_id: None,
                    reasoning_content: reasoning.clone(),
                };
                messages.push(assistant_msg);

                for tc in &tool_calls {
                    let result = if tc.name.starts_with("mcp_") {
                        // MCP 工具调用：从 server_id 前缀解析（格式: mcp_{server_id}_{tool_name}）
                        // 这里简化处理：mcp_ 前缀的工具通过 MCP 管理器执行
                        execute_mcp_tool(tc).await
                    } else {
                        tools::execute_tool(&tc.name, &tc.arguments, work_dir).await
                    };

                    // 通知前端执行结果
                    let status = if result.success { "完成" } else { "失败" };
                    let _ = channel.send(StreamChunk::TextDelta {
                        delta: format!("\n\n> **工具**: {} — {}\n> {}\n", tc.name, status, result.content),
                    });

                    messages.push(UnifiedMessage {
                        role: MessageRole::Tool,
                        content: vec![ContentBlock::Text {
                            text: result.content.clone(),
                        }],
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                        reasoning_content: None,
                    });
                }
                // 继续循环，让 LLM 基于工具结果生成回复
            }
        }
    }

    let _ = channel.send(StreamChunk::Error {
        message: "工具调用超过最大迭代次数".into(),
    });
    Ok(())
}

/// 执行 MCP 工具调用
async fn execute_mcp_tool(tc: &crate::providers::types::ToolCall) -> crate::tools::ToolResult {
    let mcp = global_mcp();
    let servers = mcp.list_servers().await;

    // 尝试匹配 MCP 服务器：工具名格式 mcp_{original_tool_name}
    // 在已启用的服务器中查找
    for server in &servers {
        if !server.enabled {
            continue;
        }
        // 尝试执行工具（服务器会匹配工具名）
        match mcp
            .execute_mcp_tool(&server.id, &tc.name, &tc.arguments)
            .await
        {
            Ok(content) => {
                return crate::tools::ToolResult {
                    success: true,
                    content,
                };
            }
            // 继续尝试下一个服务器
            Err(_) => continue,
        }
    }

    crate::tools::ToolResult {
        success: false,
        content: format!("MCP 工具 {} 执行失败：未找到可用的 MCP 服务器", tc.name),
    }
}

/// Phase 1 模拟响应
async fn simulate_demo_chat(request: &ChatRequest, channel: Channel<StreamChunk>) {
    let header = format!(
        "你好！我是 OpenAiDesktop 助手。\n\n收到你的消息，当前 Provider: **{}**, 模型: **{}**。\n\n",
        request.provider_id, request.model
    );
    let body = "这是一个演示回复。请在设置中配置 API Key 以接入真实模型。";

    for ch in (header + body).chars() {
        let _ = channel.send(StreamChunk::TextDelta {
            delta: ch.to_string(),
        });
        tokio::time::sleep(std::time::Duration::from_millis(15)).await;
    }
    let _ = channel.send(StreamChunk::Done {
        finish_reason: Some("stop".into()),
    });
}
