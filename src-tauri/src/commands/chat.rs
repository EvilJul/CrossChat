use crate::agent::tool_registry::ToolRegistry;
use crate::commands::agent_cmd::read_agent_config;
use crate::mcp::global_mcp;
use crate::providers::anthropic::AnthropicProvider;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::types::{ContentBlock, MessageRole, StreamChunk, UnifiedMessage};
use crate::providers::LlmProvider;
use crate::skills::global_skills;
use serde::Deserialize;
use tauri::ipc::Channel;

// Trait 用于抽象 Channel 行为
pub trait StreamSender: Send + Sync {
    fn send(&self, chunk: StreamChunk) -> Result<(), String>;
}

// 为 Tauri Channel 实现 trait
impl StreamSender for Channel<StreamChunk> {
    fn send(&self, chunk: StreamChunk) -> Result<(), String> {
        Channel::send(self, chunk).map_err(|e| e.to_string())
    }
}

// 为我们的适配器实现 trait
impl StreamSender for crate::commands::stream_cmd::ChannelAdapter {
    fn send(&self, chunk: StreamChunk) -> Result<(), String> {
        crate::commands::stream_cmd::ChannelAdapter::send(self, chunk)
    }
}

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
    eprintln!("[stream_chat] 开始处理请求: provider={}, model={}", request.provider_id, request.model);

    let has_creds = request.api_key.is_some() && request.api_base.is_some();

    if has_creds {
        let api_key = request.api_key.unwrap();
        let api_base = request.api_base.unwrap();
        let provider_type = request.provider_type.as_deref().unwrap_or("openai-compat");
        let work_dir = request.work_dir.unwrap_or_default();

        eprintln!("[stream_chat] 使用 Provider: {}, API Base: {}", provider_type, api_base);

        let result = match provider_type {
            "openai-compat" => {
                let provider = OpenAICompatProvider::new(api_base, api_key);
                run_agent_loop(&provider, request.messages, &request.model, channel.clone(), &work_dir).await
            }
            "anthropic" => {
                let provider = AnthropicProvider::new(api_base, api_key);
                run_agent_loop(&provider, request.messages, &request.model, channel.clone(), &work_dir).await
            }
            _ => {
                let error_msg = format!("不支持的 Provider 类型: {}", provider_type);
                eprintln!("[stream_chat] {}", error_msg);
                let _ = channel.send(StreamChunk::Error { message: error_msg.clone() });
                let _ = channel.send(StreamChunk::Done { finish_reason: Some("error".into()) });
                return Err(error_msg);
            }
        };

        match result {
            Ok(_) => {
                eprintln!("[stream_chat] 执行成功");
                Ok(())
            }
            Err(e) => {
                let error_msg = format!("Provider 错误: {}", e);
                eprintln!("[stream_chat] {}", error_msg);
                let _ = channel.send(StreamChunk::Error { message: error_msg.clone() });
                let _ = channel.send(StreamChunk::Done { finish_reason: Some("error".into()) });
                Err(error_msg)
            }
        }
    } else {
        eprintln!("[stream_chat] 使用演示模式");
        simulate_demo_chat(&request, channel).await;
        Ok(())
    }
}

/// 估算消息的 token 数量（粗略：中文 ~1.5 char/token，英文 ~3.5 char/token）
fn estimate_tokens(messages: &[UnifiedMessage]) -> usize {
    let mut total = 0usize;
    for msg in messages {
        for block in &msg.content {
            let ContentBlock::Text { text } = block else { continue; };
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

/// 自动上下文压缩（使用新 ContextCompactor 机制）
async fn auto_compress_context<T: StreamSender>(
    _provider: &dyn LlmProvider,
    messages: Vec<UnifiedMessage>,
    model: &str,
    channel: &T,
) -> Vec<UnifiedMessage> {
    let estimated = crate::agent::compactor::ContextCompactor::estimate_tokens(&messages);
    
    let model_lower = model.to_lowercase();
    let is_deepseek = model_lower.contains("deepseek");
    let soft_threshold = if is_deepseek { 980_000 } else { 16_000 };

    if estimated < soft_threshold || messages.len() < 8 {
        return messages; // 未达到阈值
    }

    // 通知前端正在进行压缩
    let _ = channel.send(StreamChunk::StatusDelta {
        message: "Context compaction in progress... folding historical dialogue".into(),
    });

    let compacted = crate::agent::compactor::ContextCompactor::compact(messages, model);
    
    let new_estimated = crate::agent::compactor::ContextCompactor::estimate_tokens(&compacted);
    let _ = channel.send(StreamChunk::StatusDelta {
        message: format!(
            "Compaction completed: {} → {} tokens (compression ratio {:.0}%)",
            estimated,
            new_estimated,
            (1.0 - new_estimated as f64 / estimated as f64) * 100.0
        ),
    });

    compacted
}

/// Agent 循环：使用 ReAct 模式智能处理任务
pub async fn run_agent_loop<T: StreamSender + Clone>(
    provider: &dyn LlmProvider,
    mut messages: Vec<UnifiedMessage>,
    model: &str,
    channel: T,
    work_dir: &str,
) -> Result<(), crate::providers::ProviderError> {
    eprintln!("[agent_loop] 步骤1: 读取 agent config");
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

    // 注入当前工作目录信息，引导 AI 使用相对路径
    if !work_dir.is_empty() {
        let dir_hint = UnifiedMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text {
                text: format!(
                    "[当前工作目录]\n你的当前工作目录是: {}\n\n\
                     重要规则:\n\
                     - 读写文件时使用相对路径（如 \"src/main.rs\"），不要使用绝对路径\n\
                     - 工具会自动将相对路径解析到当前工作目录\n\
                     - 不要在消息中输出完整的工作目录路径",
                    work_dir
                ),
            }],
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        };
        messages.insert(0, dir_hint);
    }

    eprintln!("[agent_loop] 步骤2: 初始化工具注册表");
    // 使用新的 Agent 系统
    let mut tool_registry = ToolRegistry::new();

    eprintln!("[agent_loop] 步骤3: 动态 Skill 上下文注入已移至 Agent 运行时");

    eprintln!("[agent_loop] 步骤4: 上下文压缩检查");
    // 上下文压缩：超阈值时自动摘要旧消息
    messages = auto_compress_context(provider, messages, model, &channel).await;

    eprintln!("[agent_loop] 步骤5: 加载 MCP 工具");
    // 注册 MCP 工具（带超时保护，避免不可用的 MCP 服务器阻塞聊天）
    let mcp_tools = match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        global_mcp().get_all_tools(work_dir),
    ).await {
        Ok(tools) => {
            eprintln!("[agent_loop] MCP 工具加载完成: {} 个", tools.len());
            if !tools.is_empty() {
                eprintln!("[agent_loop] MCP 工具列表:");
                for tool in &tools {
                    eprintln!("  - {} : {}", tool.name, tool.description);
                }
            }
            tools
        }
        Err(_) => {
            eprintln!("[stream_chat] MCP 工具发现超时（5s），跳过");
            let _ = channel.send(StreamChunk::StatusDelta {
                message: "MCP 工具加载超时，使用内置工具继续".into(),
            });
            vec![]
        }
    };
    tool_registry.register_batch(mcp_tools);

    eprintln!("[agent_loop] 步骤6: 启动 ReAct Agent (消息数={})", messages.len());
    // 使用 ReAct Agent 运行
    match crate::agent::react::run_react_agent(
        provider,
        messages,
        model,
        channel.clone(),
        work_dir,
        tool_registry,
    )
    .await
    {
        Ok(_) => {
            eprintln!("[Agent] 执行成功");
            Ok(())
        }
        Err(e) => {
            eprintln!("[Agent] 执行失败: {}", e);
            let _ = channel.send(StreamChunk::Error { message: e.clone() });
            Err(crate::providers::ProviderError::Other(e))
        }
    }
}

/// 执行 MCP 工具调用
#[allow(dead_code)]
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

/// 自愈 Agent：分析工具失败原因，尝试修复，然后重新执行
#[allow(dead_code)]
async fn auto_repair(
    tool_name: &str,
    arguments: &serde_json::Value,
    error: &str,
    work_dir: &str,
) -> Option<crate::tools::ToolResult> {
    let error_lower = error.to_lowercase();

    // 文件不存在 → 搜索相似文件
    if (tool_name == "read_file" || tool_name == "write_file" || tool_name == "delete_file")
        && (error_lower.contains("not found") || error_lower.contains("不存在") || error_lower.contains("no such file"))
    {
        let path = arguments["path"].as_str().unwrap_or("");
        if !path.is_empty() && !work_dir.is_empty() {
            let file_name = std::path::Path::new(path)
                .file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if !file_name.is_empty() && file_name.len() >= 2 {
                let prefix = &file_name.to_lowercase()[..2.min(file_name.len())];
                let mut similar = Vec::new();
                // 递归搜索工作目录（浅层）
                if let Ok(entries) = std::fs::read_dir(work_dir) {
                    for e in entries.flatten() {
                        let name = e.file_name().to_string_lossy().to_string();
                        if name.to_lowercase().contains(prefix) {
                            let full_path = e.path().to_string_lossy().to_string();
                            similar.push(full_path);
                        }
                    }
                }
                if !similar.is_empty() {
                    // 返回相对路径，避免 AI 记住绝对路径
                    let relative_similar: Vec<String> = similar.iter().filter_map(|s| {
                        std::path::Path::new(s).file_name().map(|n| n.to_string_lossy().to_string())
                    }).collect();
                    return Some(crate::tools::ToolResult {
                        success: true,
                        content: format!(
                            "> 🔧 文件 '{}' 未找到。当前目录中相似文件:\n{}\n> 请使用正确路径重试。",
                            path,
                            relative_similar.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n")
                        ),
                    });
                }
            }
        }
        return Some(crate::tools::ToolResult {
            success: false,
            content: "> 文件未找到。请用 list_directory 确认文件路径后重试。".into(),
        });
    }

    // Python 模块缺失 → 已由 python_sandbox 处理，直接重试读/写/命令
    if error_lower.contains("modulenotfound") || error_lower.contains("no module named") {
        return Some(crate::tools::ToolResult {
            success: true,
            content: "> 🔧 Python 模块缺失，沙盒已自动安装。请重试原命令。".into(),
        });
    }

    // 权限拒绝
    if error_lower.contains("permission denied") || error_lower.contains("access denied")
        || error_lower.contains("eacces")
    {
        return Some(crate::tools::ToolResult {
            success: false,
            content: "> 权限不足。请尝试:\n> 1. 换一个工作目录\n> 2. 检查文件是否被其他程序占用\n> 3. 使用 list_directory 确认路径".into(),
        });
    }

    // 命令语法错误 → 提取错误信息
    if tool_name == "run_command" && (error_lower.contains("syntax") || error_lower.contains("unexpected")) {
        return Some(crate::tools::ToolResult {
            success: false,
            content: format!("> 命令执行出错:\n> {}\n> 请修正语法后重试。", error.lines().next().unwrap_or(error)),
        });
    }

    // 其他错误 → 返回错误信息让 LLM 自主决策
    None
}

/// Phase 1 模拟响应
async fn simulate_demo_chat(request: &ChatRequest, channel: Channel<StreamChunk>) {
    let header = format!(
        "你好！我是 CrossChat 助手。\n\n收到你的消息，当前 Provider: **{}**, 模型: **{}**。\n\n",
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
