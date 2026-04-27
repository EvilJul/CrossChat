use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Checkpoint {
    pub messages: Vec<CheckpointMessage>,
    pub provider_id: String,
    pub model: String,
    pub work_dir: String,
    pub saved_at: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CheckpointMessage {
    pub role: String,
    pub content: String,
    pub thinking: Option<String>,
}

fn checkpoint_path() -> PathBuf {
    let home = std::env::var("APPDATA")
        .or_else(|_| std::env::var("HOME"))
        .unwrap_or_else(|_| ".".into());
    let dir = PathBuf::from(home).join(".crosschat");
    std::fs::create_dir_all(&dir).ok();
    dir.join("checkpoint.json")
}

/// 保存当前对话检查点（中断时调用）
#[tauri::command]
pub fn save_checkpoint(checkpoint: Checkpoint) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&checkpoint).map_err(|e| e.to_string())?;
    std::fs::write(checkpoint_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

/// 读取上次中断的检查点
#[tauri::command]
pub fn load_checkpoint() -> Result<Option<Checkpoint>, String> {
    let path = checkpoint_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let checkpoint: Checkpoint = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(Some(checkpoint))
}

/// 清除检查点（继续成功后调用）
#[tauri::command]
pub fn clear_checkpoint() -> Result<(), String> {
    let path = checkpoint_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| e.to_string())?;
    }
    Ok(())
}
