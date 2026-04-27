use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub meta: SessionMeta,
    pub messages: Vec<SessionMessage>,
    #[serde(default)]
    pub summary: Option<String>, // 上下文压缩摘要
}

fn sessions_dir() -> PathBuf {
    let home = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(home).join(".crosschat").join("sessions");
    fs::create_dir_all(&dir).ok();
    dir
}

fn session_path(id: &str) -> PathBuf {
    sessions_dir().join(format!("{}.json", id))
}

/// 创建新会话
#[tauri::command]
pub fn create_session(title: String) -> Result<SessionMeta, String> {
    let id = format!("session-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let meta = SessionMeta {
        id: id.clone(),
        title,
        created_at: now,
        updated_at: now,
        message_count: 0,
    };

    let session = Session {
        meta: meta.clone(),
        messages: vec![],
        summary: None,
    };

    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(session_path(&id), json).map_err(|e| e.to_string())?;
    Ok(meta)
}

/// 列出所有会话
#[tauri::command]
pub fn list_sessions() -> Result<Vec<SessionMeta>, String> {
    let dir = sessions_dir();
    let mut sessions = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(session) = serde_json::from_str::<Session>(&content) {
                    sessions.push(session.meta);
                }
            }
        }
    }
    sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(sessions)
}

/// 获取会话消息
#[tauri::command]
pub fn get_session(id: String) -> Result<Session, String> {
    let content = fs::read_to_string(session_path(&id)).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// 保存消息到会话
#[tauri::command]
pub fn save_messages(
    session_id: String,
    messages: Vec<SessionMessage>,
    summary: Option<String>,
) -> Result<(), String> {
    let path = session_path(&session_id);
    let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut session: Session = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    session.messages = messages;
    session.summary = summary;
    session.meta.updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    session.meta.message_count = session.messages.len();

    // 自动更新标题（用第一条用户消息的前30个字符）
    if session.meta.title.is_empty() || session.meta.title == "新对话" {
        if let Some(first) = session.messages.iter().find(|m| m.role == "user") {
            let title: String = first.content.chars().take(30).collect();
            session.meta.title = if first.content.len() > 30 {
                format!("{}...", title)
            } else {
                title
            };
        }
    }

    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

/// 删除会话
#[tauri::command]
pub fn delete_session(id: String) -> Result<(), String> {
    let path = session_path(&id);
    if path.exists() {
        fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
