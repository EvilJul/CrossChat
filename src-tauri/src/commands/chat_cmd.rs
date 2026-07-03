use chrono::Utc;
use futures_util::StreamExt;
use std::sync::Arc;
use tauri::State;

use crate::adapters::model::OpenAIClient;
use crate::adapters::store::SqliteThreadStore;
use crate::adapters::tool::LocalToolHost;
use crate::core::models::{ContentPart, Message, MessageRole, ToolCall, Turn, TurnItem};
use crate::error::AppError;
use crate::ports::model_client::{ModelClient, ModelRequest, StreamChunk};
use crate::ports::tool_host::ToolHost;
use crate::ports::ThreadStore;

/// 本期允许 LLM 调用的工具白名单（只读）。
/// write_file / delete_file / run_command 等写操作暂不开放，
/// 待 ApprovalGate 审批门接线后再放开（见 Obsidian 待办 P1）。
const ALLOWED_TOOLS: &[&str] = &["read_file", "list_directory"];

/// ReAct 循环的最大迭代轮数，防止工具调用死循环。
const MAX_TOOL_ITERATIONS: usize = 8;

/// 流式增量事件：通过 Tauri Channel 推给前端。
///
/// 序列化为「内部标签式」JSON，标签字段名为 `type`，标签值 camelCase：
/// `{ "type": "delta", "text": "..." }` / `{ "type": "reasoning", "text": "..." }`
/// / `{ "type": "done", "text": "<完整文本>" }` / `{ "type": "error", "message": "..." }`。
/// 该契约由前端冻结，不可随意更改。
#[derive(Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StreamEvent {
    /// 文本增量。
    Delta { text: String },
    /// 推理（reasoning_content）增量。
    Reasoning { text: String },
    /// 流结束，携带完整文本。
    Done { text: String },
    /// 出错，携带可读中文错误信息。
    Error { message: String },
}

/// 校验 API base URL：去空格后非空，且以 `http://` 或 `https://` 开头。
/// 用简单前缀判断，不引入 regex/url crate。
fn validate_api_base(api_base: &str) -> Result<(), AppError> {
    let trimmed = api_base.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidInput("API 地址不能为空".to_string()));
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(AppError::InvalidInput(
            "API 地址必须以 http:// 或 https:// 开头".to_string(),
        ));
    }
    Ok(())
}

#[tauri::command]
pub async fn fetch_models(
    api_key: String,
    api_base: String,
) -> Result<Vec<String>, AppError> {
    validate_api_base(&api_base)?;

    let base = api_base.trim_end_matches('/');
    let url = format!("{}/models", base);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| AppError::Network(format!("请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Api(format!("API 错误: {}", text)));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(format!("解析响应失败: {}", e)))?;
    let models = data["data"]
        .as_array()
        .ok_or_else(|| AppError::Parse("响应格式错误: 未找到 data 字段".to_string()))?
        .iter()
        .filter_map(|m| m["id"].as_str().map(String::from))
        .collect();

    Ok(models)
}

/// 把历史 TurnItem 序列展开成模型可消费的 Message 序列（含工具调用/结果的多轮上下文）。
///
/// - UserMessage → user
/// - AssistantText → assistant（纯文本）
/// - ToolCall → assistant（携带 tool_calls）
/// - ToolResult → tool（携带 tool_use_id）
/// - AssistantReasoning / Compaction / Approval / Error 不进模型上下文（推理内容不回传）
fn history_to_messages(history_items: &[TurnItem], system_prompt: Option<String>) -> Vec<Message> {
    let mut messages: Vec<Message> = Vec::new();
    if let Some(sys) = system_prompt {
        if !sys.trim().is_empty() {
            messages.push(Message::system(sys));
        }
    }

    for item in history_items {
        match item {
            TurnItem::UserMessage { text, .. } => {
                messages.push(Message::user(text.clone()));
            }
            TurnItem::AssistantText { text } => {
                if !text.is_empty() {
                    messages.push(Message::assistant(text.clone()));
                }
            }
            TurnItem::ToolCall { id, name, args } => {
                messages.push(Message {
                    role: MessageRole::Assistant,
                    content: vec![ContentPart::ToolUse {
                        id: id.clone(),
                        name: name.clone(),
                        input: args.clone(),
                    }],
                });
            }
            TurnItem::ToolResult { call_id, result, error } => {
                let (content, is_error) = match error {
                    Some(e) => (e.clone(), true),
                    None => (result.clone(), false),
                };
                messages.push(Message {
                    role: MessageRole::User,
                    content: vec![ContentPart::ToolResult {
                        tool_use_id: call_id.clone(),
                        content,
                        is_error,
                    }],
                });
            }
            _ => {}
        }
    }
    messages
}

/// 累积中的工具调用（跨流式增量拼接）。
#[derive(Default, Clone)]
struct PendingToolCall {
    id: String,
    name: String,
    args_json: String,
}

#[tauri::command]
pub async fn send_chat_message(
    store: State<'_, Arc<SqliteThreadStore>>,
    session_id: String,
    user_text: String,
    api_key: String,
    model: String,
    api_base: String,
    on_event: tauri::ipc::Channel<StreamEvent>,
) -> Result<String, AppError> {
    // 关键输入校验：用户消息非空、API 地址合法、模型名非空。
    if user_text.trim().is_empty() {
        return Err(AppError::InvalidInput("消息内容不能为空".to_string()));
    }
    validate_api_base(&api_base)?;
    if model.trim().is_empty() {
        return Err(AppError::InvalidInput("模型名不能为空".to_string()));
    }

    let thread = store
        .get_thread(&session_id)
        .await
        .map_err(|e| AppError::Storage(e.to_string()))?;

    let turns = store.list_turns(&session_id, 100).await.unwrap_or_default();

    // 收集本次用户消息之前的完整历史 items（单 Turn 累积方案）。
    // list_turns 已按 rowid（插入序）排序，这里再按 started_at 兜底排序后展平。
    let mut sorted_turns = turns.clone();
    sorted_turns.sort_by_key(|t| t.started_at);
    let mut running_items: Vec<TurnItem> = sorted_turns
        .iter()
        .flat_map(|t| t.items.clone())
        .collect();

    // 追加本次用户消息。running_items 会随 ReAct 循环不断增长（追加 ToolCall/ToolResult/AssistantText）。
    running_items.push(TurnItem::UserMessage {
        text: user_text.clone(),
        attachments: vec![],
    });

    // 准备模型客户端与工具主机（只读工具）。
    let base = api_base.trim_end_matches('/').to_string();
    let client = OpenAIClient::new(api_key.clone()).with_base_url(base);
    let tool_host = LocalToolHost::with_default();

    // 只暴露白名单内的只读工具给模型。
    let all_tools = tool_host.list_tools().await;
    let tools: Vec<_> = all_tools
        .into_iter()
        .filter(|t| ALLOWED_TOOLS.contains(&t.name.as_str()))
        .collect();

    let system_prompt = Some(
        "你是 CrossChat 智能助手。你可以使用 read_file、list_directory 工具读取本地文件与目录来协助用户。\
         调用工具前先说明意图，得到结果后据实回答。"
            .to_string(),
    );

    // 本轮对外可见的最终回复文本（最后一次无工具调用的 assistant 文本）。
    let mut final_text = String::new();
    // 本轮推理内容（用于落库；每轮迭代会覆盖为最近一次）。
    let mut last_reasoning = String::new();

    // ReAct 循环：流式生成 → 若有工具调用则执行并回填 → 再次生成，直到无工具调用或达上限。
    for iteration in 0..MAX_TOOL_ITERATIONS {
        let messages = history_to_messages(&running_items, system_prompt.clone());
        let request = ModelRequest {
            messages,
            model: model.clone(),
            tools: tools.clone(),
            max_tokens: None,
            temperature: None,
        };

        let mut stream = client
            .stream_completion(request)
            .await
            .map_err(|e| {
                let msg = format!("模型请求失败: {}", e);
                let _ = on_event.send(StreamEvent::Error { message: msg.clone() });
                AppError::Api(msg)
            })?;

        let mut iter_text = String::new();
        let mut iter_reasoning = String::new();
        // index → 累积的工具调用
        let mut pending_tools: std::collections::BTreeMap<usize, PendingToolCall> =
            std::collections::BTreeMap::new();
        // ToolCallStart 只带 id/name，需要一个从 id 找回累积槽的映射。
        let mut id_to_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut next_index: usize = 0;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| {
                let msg = format!("读取流失败: {}", e);
                let _ = on_event.send(StreamEvent::Error { message: msg.clone() });
                AppError::Network(msg)
            })?;

            match chunk {
                StreamChunk::TextDelta { text } => {
                    if !text.is_empty() {
                        iter_text.push_str(&text);
                        let _ = on_event.send(StreamEvent::Delta { text });
                    }
                }
                StreamChunk::ReasoningDelta { text } => {
                    if !text.is_empty() {
                        iter_reasoning.push_str(&text);
                        let _ = on_event.send(StreamEvent::Reasoning { text });
                    }
                }
                StreamChunk::ToolCallStart { id, name } => {
                    let idx = next_index;
                    next_index += 1;
                    id_to_index.insert(id.clone(), idx);
                    pending_tools.insert(idx, PendingToolCall { id, name, args_json: String::new() });
                }
                StreamChunk::ToolCallArgsDelta { id, args_json } => {
                    if let Some(&idx) = id_to_index.get(&id) {
                        if let Some(pt) = pending_tools.get_mut(&idx) {
                            pt.args_json.push_str(&args_json);
                        }
                    }
                }
                StreamChunk::ToolCallComplete { .. } => {}
                StreamChunk::Done { .. } => {}
            }
        }

        if !iter_reasoning.is_empty() {
            last_reasoning = iter_reasoning.clone();
        }

        // 无工具调用：本轮 assistant 文本即最终回复，结束循环。
        if pending_tools.is_empty() {
            final_text = iter_text.clone();
            running_items.push(TurnItem::AssistantText { text: iter_text });
            break;
        }

        // 有工具调用：先把 assistant 文本（可能为空）与工具调用记入上下文。
        if !iter_text.is_empty() {
            running_items.push(TurnItem::AssistantText { text: iter_text });
        }

        // 逐个执行工具，把 ToolCall 与 ToolResult 记入 running_items 供下一轮回填。
        for (_, pt) in pending_tools.iter() {
            let args: serde_json::Value =
                serde_json::from_str(&pt.args_json).unwrap_or(serde_json::json!({}));

            running_items.push(TurnItem::ToolCall {
                id: pt.id.clone(),
                name: pt.name.clone(),
                args: args.clone(),
            });

            // 白名单二次防线：即使模型硬调非白名单工具，也拒绝执行。
            let (result_text, err_text) = if !ALLOWED_TOOLS.contains(&pt.name.as_str()) {
                (String::new(), Some(format!("工具 {} 未被允许调用", pt.name)))
            } else {
                let call = ToolCall {
                    id: pt.id.clone(),
                    name: pt.name.clone(),
                    arguments: args,
                };
                match tool_host.execute(&call).await {
                    Ok(tr) => match tr.error {
                        Some(e) => (String::new(), Some(e)),
                        None => (tr.output, None),
                    },
                    Err(e) => (String::new(), Some(e.to_string())),
                }
            };

            running_items.push(TurnItem::ToolResult {
                call_id: pt.id.clone(),
                result: result_text,
                error: err_text,
            });
        }

        // 达到迭代上限仍在调工具：给出提示并结束。
        if iteration == MAX_TOOL_ITERATIONS - 1 {
            final_text = "（工具调用达到上限，已停止）".to_string();
            running_items.push(TurnItem::AssistantText { text: final_text.clone() });
        }
    }

    // 通知前端流结束，携带完整文本。
    let _ = on_event.send(StreamEvent::Done { text: final_text.clone() });

    // 单 Turn 累积方案：把「历史 + 本轮（含工具调用/结果）」重写进唯一一个 Turn。
    let _ = store.delete_turns_for_thread(&session_id).await;

    // running_items 已包含：历史 + 本次 user + 本轮 assistant/tool 交互全过程。
    // 但推理内容（AssistantReasoning）需单独插入到最终 assistant 文本之前。
    let mut items = running_items;
    if !last_reasoning.is_empty() {
        // 在最后一条 AssistantText 之前插入推理项（找不到则追加到末尾）。
        if let Some(pos) = items.iter().rposition(|i| matches!(i, TurnItem::AssistantText { .. })) {
            items.insert(pos, TurnItem::AssistantReasoning { content: last_reasoning.clone() });
        } else {
            items.push(TurnItem::AssistantReasoning { content: last_reasoning.clone() });
        }
    }

    let mut turn = Turn::new(session_id.clone(), model.clone());
    for item in items {
        turn.add_item(item);
    }
    turn.complete(Default::default());

    store.save_turn(&turn).await.map_err(|e| AppError::Storage(e.to_string()))?;

    let mut updated = thread;
    updated.updated_at = Utc::now();
    if updated.title.is_empty() || updated.title == "新对话" {
        let t: String = user_text.chars().take(30).collect();
        updated.title = if user_text.len() > 30 {
            format!("{}...", t)
        } else {
            t
        };
    }
    let _ = store.update_thread(&updated).await;

    Ok(final_text)
}
