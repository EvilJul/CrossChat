use chrono::Utc;
use futures_util::StreamExt;
use std::sync::Arc;
use tauri::State;

use crate::adapters::store::SqliteThreadStore;
use crate::core::models::{Turn, TurnItem};
use crate::error::AppError;
use crate::ports::ThreadStore;

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
    // list_turns 按随机 UUID 排序，顺序不可靠，这里显式按 started_at 升序排序后再展平，
    // 保证历史顺序正确。理论上历史只会有一个 Turn（本方案始终重写单 Turn），
    // 但为兼容多 Turn 的旧数据仍做排序展平。
    let mut sorted_turns = turns.clone();
    sorted_turns.sort_by_key(|t| t.started_at);
    let history_items: Vec<TurnItem> = sorted_turns
        .iter()
        .flat_map(|t| t.items.clone())
        .collect();

    let mut messages: Vec<serde_json::Value> = Vec::new();
    for turn in &turns {
        for item in &turn.items {
            match item {
                TurnItem::UserMessage { text, .. } => {
                    messages.push(serde_json::json!({"role": "user", "content": text}));
                }
                TurnItem::AssistantText { text } => {
                    messages.push(serde_json::json!({"role": "assistant", "content": text}));
                }
                _ => {}
            }
        }
    }
    messages.push(serde_json::json!({"role": "user", "content": user_text}));

    let base = api_base.trim_end_matches('/');
    let url = format!("{}/chat/completions", base);

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": true,
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("API 请求失败: {}", e)))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(AppError::Api(format!("API 错误 ({}): {}", status, text)));
    }

    // 逐块读取 SSE 流：边收边通过 Channel 推送增量，同时累积完整文本用于落库。
    let mut full_text = String::new();
    let mut full_reasoning = String::new();
    let mut buffer = String::new();
    let mut stream = resp.bytes_stream();

    'outer: while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::Network(format!("读取流失败: {}", e)))?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // 按 '\n' 逐行处理，保留最后不完整的一行在 buffer 里等下一个 chunk。
        loop {
            let nl = match buffer.find('\n') {
                Some(i) => i,
                None => break,
            };
            let line = buffer[..nl].trim().to_string();
            buffer.drain(..=nl);

            if line.is_empty() {
                continue;
            }
            // SSE 行形如 `data: {json}`；只处理 data: 行。
            let data = match line.strip_prefix("data:") {
                Some(d) => d.trim(),
                None => continue,
            };
            if data == "[DONE]" {
                break 'outer;
            }
            // 单行解析失败就跳过该行，不中断整个流。
            let json: serde_json::Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let delta = &json["choices"][0]["delta"];
            if let Some(content) = delta["content"].as_str() {
                if !content.is_empty() {
                    full_text.push_str(content);
                    let _ = on_event.send(StreamEvent::Delta { text: content.to_string() });
                }
            }
            if let Some(reasoning) = delta["reasoning_content"].as_str() {
                if !reasoning.is_empty() {
                    full_reasoning.push_str(reasoning);
                    let _ = on_event.send(StreamEvent::Reasoning { text: reasoning.to_string() });
                }
            }
        }
    }

    // 通知前端流结束，携带完整文本。
    let _ = on_event.send(StreamEvent::Done { text: full_text.clone() });

    // 单 Turn 累积方案：始终把「历史 + 本轮」重写进唯一一个 Turn。
    // items 在 Turn 的 JSON 数组里天然有序，彻底避开 list_turns 的随机 UUID 排序问题。
    let _ = store.delete_turns_for_thread(&session_id).await;

    // 历史（有序）+ 本次用户消息 + 可选推理 + 本次 AI 回复
    let mut items = history_items;

    items.push(TurnItem::UserMessage {
        text: user_text.clone(),
        attachments: vec![],
    });

    if !full_reasoning.is_empty() {
        items.push(TurnItem::AssistantReasoning {
            content: full_reasoning.clone(),
        });
    }

    items.push(TurnItem::AssistantText {
        text: full_text.clone(),
    });

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

    Ok(full_text)
}
