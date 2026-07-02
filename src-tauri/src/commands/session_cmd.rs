use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::State;

use crate::adapters::store::SqliteThreadStore;
use crate::core::models::{Thread, ThreadMode, ThreadStatus, Turn, TurnItem};
use crate::ports::ThreadStore;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: usize,
    // 会话状态："active" / "archived" / "deleted"
    pub status: String,
    // 是否置顶
    pub pinned: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    #[serde(default)]
    pub thinking: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub meta: SessionMeta,
    pub messages: Vec<SessionMessage>,
    #[serde(default)]
    pub summary: Option<String>,
}

fn meta_from_thread(thread: &Thread, message_count: usize) -> SessionMeta {
    SessionMeta {
        id: thread.id.clone(),
        title: thread.title.clone(),
        created_at: thread.created_at.timestamp() as u64,
        updated_at: thread.updated_at.timestamp() as u64,
        message_count,
        status: format!("{:?}", thread.status).to_lowercase(),
        pinned: thread.pinned,
    }
}

fn turn_item_to_message(item: &TurnItem, ts: u64) -> Option<SessionMessage> {
    match item {
        TurnItem::UserMessage { text, .. } => Some(SessionMessage {
            role: "user".into(),
            content: text.clone(),
            timestamp: ts,
            thinking: None,
            tool_name: None,
        }),
        TurnItem::AssistantText { text } => Some(SessionMessage {
            role: "assistant".into(),
            content: text.clone(),
            timestamp: ts,
            thinking: None,
            tool_name: None,
        }),
        TurnItem::AssistantReasoning { content } => Some(SessionMessage {
            role: "assistant".into(),
            content: String::new(),
            timestamp: ts,
            thinking: Some(content.clone()),
            tool_name: None,
        }),
        TurnItem::ToolCall { name, args, .. } => Some(SessionMessage {
            role: "tool_call".into(),
            content: args.to_string(),
            timestamp: ts,
            thinking: None,
            tool_name: Some(name.clone()),
        }),
        TurnItem::ToolResult { result, error, .. } => Some(SessionMessage {
            role: "tool_result".into(),
            content: error.clone().unwrap_or(result.clone()),
            timestamp: ts,
            thinking: None,
            tool_name: None,
        }),
        _ => None,
    }
}

#[tauri::command]
pub async fn create_session(
    store: State<'_, Arc<SqliteThreadStore>>,
    title: String,
) -> Result<SessionMeta, String> {
    let thread = Thread::new(title, None, ThreadMode::Chat);
    store.create_thread(&thread).await.map_err(|e| e.to_string())?;
    Ok(meta_from_thread(&thread, 0))
}

#[tauri::command]
pub async fn list_sessions(
    store: State<'_, Arc<SqliteThreadStore>>,
) -> Result<Vec<SessionMeta>, String> {
    let threads = store.list_threads(100, 0).await.map_err(|e| e.to_string())?;
    let mut metas = Vec::with_capacity(threads.len());
    for thread in &threads {
        let turns = store.list_turns(&thread.id, 100).await.map_err(|e| e.to_string())?;
        let count: usize = turns.iter().map(|t| {
            t.items.iter().filter(|i| {
                matches!(i, TurnItem::UserMessage { .. } | TurnItem::AssistantText { .. })
            }).count()
        }).sum();
        metas.push(meta_from_thread(thread, count));
    }
    Ok(metas)
}

#[tauri::command]
pub async fn get_session(
    store: State<'_, Arc<SqliteThreadStore>>,
    id: String,
) -> Result<Session, String> {
    let thread = store.get_thread(&id).await.map_err(|e| e.to_string())?;
    let turns = store.list_turns(&id, 100).await.map_err(|e| e.to_string())?;

    let mut messages = Vec::new();
    let mut summary = None;

    for turn in &turns {
        let ts = turn.started_at.timestamp() as u64;
        for item in &turn.items {
            if let TurnItem::Compaction { summary: s, .. } = item {
                summary = Some(s.clone());
            } else if let Some(msg) = turn_item_to_message(item, ts) {
                messages.push(msg);
            }
        }
    }

    let message_count = messages.len();

    Ok(Session {
        meta: meta_from_thread(&thread, message_count),
        messages,
        summary,
    })
}

#[tauri::command]
pub async fn save_messages(
    store: State<'_, Arc<SqliteThreadStore>>,
    session_id: String,
    messages: Vec<SessionMessage>,
    summary: Option<String>,
) -> Result<(), String> {
    let thread = store.get_thread(&session_id).await.map_err(|e| e.to_string())?;

    let _ = store.delete_turns_for_thread(&session_id).await;

    let mut items: Vec<TurnItem> = Vec::with_capacity(messages.len());

    for msg in &messages {
        match msg.role.as_str() {
            "user" => items.push(TurnItem::UserMessage {
                text: msg.content.clone(),
                attachments: vec![],
            }),
            "assistant" => items.push(TurnItem::AssistantText {
                text: msg.content.clone(),
            }),
            _ => {}
        }
    }

    if let Some(s) = summary {
        if !s.is_empty() {
            items.push(TurnItem::Compaction {
                summary: s,
                removed_count: 0,
                digest: String::new(),
            });
        }
    }

    let mut turn = Turn::new(session_id.clone(), "claude-3-5-sonnet-20241022".into());
    for item in items {
        turn.add_item(item);
    }
    turn.complete(Default::default());

    store.save_turn(&turn).await.map_err(|e| e.to_string())?;

    let mut updated = thread;
    updated.updated_at = Utc::now();
    if (updated.title.is_empty() || updated.title == "新对话") && !messages.is_empty() {
        if let Some(first) = messages.iter().find(|m| m.role == "user") {
            let t: String = first.content.chars().take(30).collect();
            updated.title = if first.content.len() > 30 {
                format!("{}...", t)
            } else {
                t
            };
        }
    }
    let _ = store.update_thread(&updated).await;

    Ok(())
}

#[tauri::command]
pub async fn delete_session(
    store: State<'_, Arc<SqliteThreadStore>>,
    id: String,
) -> Result<(), String> {
    store.delete_thread(&id).await.map_err(|e| e.to_string())
}

/// 设置会话状态（归档 / 取消归档）。
/// status 仅接受 "active" 或 "archived"，其它值返回 Err。
/// 归档不应影响列表按更新时间排序，因此**不更新 updated_at**（复用原值写回）。
#[tauri::command]
pub async fn set_session_status(
    store: State<'_, Arc<SqliteThreadStore>>,
    id: String,
    status: String,
) -> Result<(), String> {
    let new_status = match status.as_str() {
        "active" => ThreadStatus::Active,
        "archived" => ThreadStatus::Archived,
        other => return Err(format!("非法的会话状态: {}", other)),
    };
    let mut thread = store.get_thread(&id).await.map_err(|e| e.to_string())?;
    thread.status = new_status;
    // 保留原 updated_at，不刷新
    store.update_thread(&thread).await.map_err(|e| e.to_string())
}

/// 重命名会话。title 去首尾空格；为空返回 Err；会更新 updated_at。
#[tauri::command]
pub async fn rename_session(
    store: State<'_, Arc<SqliteThreadStore>>,
    id: String,
    title: String,
) -> Result<(), String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err("标题不能为空".to_string());
    }
    let mut thread = store.get_thread(&id).await.map_err(|e| e.to_string())?;
    thread.title = trimmed.to_string();
    thread.updated_at = Utc::now();
    store.update_thread(&thread).await.map_err(|e| e.to_string())
}

/// 置顶 / 取消置顶会话。**不更新 updated_at**（复用原值写回）。
#[tauri::command]
pub async fn set_session_pinned(
    store: State<'_, Arc<SqliteThreadStore>>,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    let mut thread = store.get_thread(&id).await.map_err(|e| e.to_string())?;
    thread.pinned = pinned;
    // 保留原 updated_at，不刷新
    store.update_thread(&thread).await.map_err(|e| e.to_string())
}
