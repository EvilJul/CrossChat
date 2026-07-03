use std::path::PathBuf;
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::core::models::{Thread, Turn, TurnItem, ThreadMode, ThreadStatus, TurnStatus, TokenUsage};
use crate::ports::ThreadStore;

#[derive(Debug, Deserialize)]
struct OldSession {
    id: String,
    title: Option<String>,
    messages: Vec<OldMessage>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OldMessage {
    role: String,
    content: String,
    #[serde(default)]
    timestamp: Option<String>,
}

pub async fn migrate_sessions<T: ThreadStore>(store: &T) -> Result<MigrationReport, MigrationError> {
    // 读旧栈数据：旧版 CrossChat 在 Unix 上把 sessions 存于 $HOME/.crosschat/sessions，
    // 故此处用 home_dir() 读取——macOS/Linux 正确。写入新库则用 data_dir()（见 migrate_data /
    // lib.rs）：读旧路径、写新路径的不对称是【有意为之】，勿改。
    // ⚠️ Windows 例外：旧版在 Windows 存于 %APPDATA%\.crosschat，而 home_dir() 在 Windows 是
    //   %USERPROFILE%，二者不同 → 旧 Windows 数据可能读不到。修此需在 Windows + 旧数据上验证，
    //   当前环境无法测试，暂保持不变（仅影响从旧版升级的 Windows 用户找回历史会话）。
    let sessions_dir = dirs::home_dir()
        .ok_or(MigrationError::NoHomeDir)?
        .join(".crosschat/sessions");

    if !sessions_dir.exists() {
        return Ok(MigrationReport { total: 0, success: 0, errors: vec![] });
    }

    let backup_dir = sessions_dir.parent().unwrap().join("sessions_backup");
    fs::create_dir_all(&backup_dir).map_err(|e| MigrationError::BackupFailed(e.to_string()))?;

    let entries = fs::read_dir(&sessions_dir).map_err(|e| MigrationError::ReadFailed(e.to_string()))?;
    let mut report = MigrationReport { total: 0, success: 0, errors: vec![] };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        report.total += 1;

        match migrate_single_session(&path, store).await {
            Ok(_) => {
                report.success += 1;
                let backup_path = backup_dir.join(path.file_name().unwrap());
                let _ = fs::copy(&path, backup_path);
            }
            Err(e) => report.errors.push(format!("{}: {}", path.display(), e)),
        }
    }

    Ok(report)
}

async fn migrate_single_session<T: ThreadStore>(path: &PathBuf, store: &T) -> Result<(), MigrationError> {
    let content = fs::read_to_string(path).map_err(|e| MigrationError::ReadFailed(e.to_string()))?;
    let old: OldSession = serde_json::from_str(&content).map_err(|e| MigrationError::ParseFailed(e.to_string()))?;

    let now = Utc::now();
    let created_at = old.created_at.as_ref()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or(now);

    let thread = Thread {
        id: old.id.clone(),
        title: old.title.unwrap_or_else(|| "Migrated Session".to_string()),
        workspace_path: None,
        status: ThreadStatus::Archived,
        mode: ThreadMode::Chat,
        goal: None,
        created_at,
        updated_at: now,
        pinned: false,
    };

    store.create_thread(&thread).await.map_err(|e| MigrationError::StoreFailed(e.to_string()))?;

    if !old.messages.is_empty() {
        let mut turn = Turn::new(old.id.clone(), "claude-3-5-sonnet-20241022".to_string());

        for msg in old.messages {
            let item = match msg.role.as_str() {
                "user" => TurnItem::UserMessage { text: msg.content, attachments: vec![] },
                "assistant" => TurnItem::AssistantText { text: msg.content },
                _ => continue,
            };
            turn.add_item(item);
        }

        turn.complete(TokenUsage::default());
        store.save_turn(&turn).await.map_err(|e| MigrationError::StoreFailed(e.to_string()))?;
    }

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct MigrationReport {
    pub total: usize,
    pub success: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("No home directory")]
    NoHomeDir,
    #[error("Backup failed: {0}")]
    BackupFailed(String),
    #[error("Read failed: {0}")]
    ReadFailed(String),
    #[error("Parse failed: {0}")]
    ParseFailed(String),
    #[error("Store failed: {0}")]
    StoreFailed(String),
}

#[tauri::command]
pub async fn migrate_data() -> Result<MigrationReport, String> {
    let data_dir = dirs::data_dir()
        .ok_or("Cannot find data directory")?
        .join(".crosschat");
    let db_path = data_dir.join("threads.db");
    let db_url = format!("sqlite:{}", db_path.display());

    let store = crate::adapters::store::SqliteThreadStore::new(&db_url)
        .map_err(|e| e.to_string())?;

    migrate_sessions(&store).await.map_err(|e| e.to_string())
}
