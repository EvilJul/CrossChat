use chrono::Utc;
use std::sync::Arc;
use tauri::State;

use crate::adapters::store::SqliteThreadStore;
use crate::core::models::{Turn, TurnItem};
use crate::ports::ThreadStore;

#[tauri::command]
pub async fn fetch_models(
    api_key: String,
    api_base: String,
) -> Result<Vec<String>, String> {
    let base = api_base.trim_end_matches('/');
    let url = format!("{}/models", base);

    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if !resp.status().is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("API 错误: {}", text));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;
    let models = data["data"]
        .as_array()
        .ok_or("响应格式错误: 未找到 data 字段")?
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
) -> Result<String, String> {
    let thread = store.get_thread(&session_id).await.map_err(|e| e.to_string())?;

    let turns = store.list_turns(&session_id, 100).await.unwrap_or_default();

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
        "stream": false,
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("API 请求失败: {}", e))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("API 错误 ({}): {}", status, text));
    }

    let data: serde_json::Value = resp.json().await.map_err(|e| format!("解析响应失败: {}", e))?;

    let msg = &data["choices"][0]["message"];
    let assistant_text = msg["content"]
        .as_str()
        .ok_or("API 返回格式错误: 未找到 choices[0].message.content")?
        .to_string();

    let reasoning = msg["reasoning_content"].as_str().map(|s| s.to_string());

    let _ = store.delete_turns_for_thread(&session_id).await;

    let mut items = Vec::new();

    items.push(TurnItem::UserMessage {
        text: user_text.clone(),
        attachments: vec![],
    });

    if let Some(ref r) = reasoning {
        if !r.is_empty() {
            items.push(TurnItem::AssistantReasoning { content: r.clone() });
        }
    }

    items.push(TurnItem::AssistantText {
        text: assistant_text.clone(),
    });

    let mut turn = Turn::new(session_id.clone(), model.clone());
    for item in items {
        turn.add_item(item);
    }
    turn.complete(Default::default());

    store.save_turn(&turn).await.map_err(|e| e.to_string())?;

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

    Ok(assistant_text)
}
