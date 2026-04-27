use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// 列出目录下的文件和子目录
#[tauri::command]
pub async fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = PathBuf::from(&path);
    if !dir.exists() {
        return Err(format!("路径不存在: {}", path));
    }
    if !dir.is_dir() {
        return Err(format!("不是目录: {}", path));
    }

    let mut entries = Vec::new();
    let read_dir = std::fs::read_dir(&dir).map_err(|e| format!("无法读取目录: {}", e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("读取条目失败: {}", e))?;
        let metadata = entry.metadata().map_err(|e| format!("获取元数据失败: {}", e))?;
        let name = entry.file_name().to_string_lossy().to_string();

        // 跳过隐藏文件
        if name.starts_with('.') {
            continue;
        }

        entries.push(FileEntry {
            name: name.clone(),
            path: entry.path().to_string_lossy().to_string(),
            is_dir: metadata.is_dir(),
            size: metadata.len(),
        });
    }

    // 目录在前，文件在后，按名称排序
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));

    Ok(entries)
}

/// 获取当前用户的主目录
#[tauri::command]
pub fn get_home_dir() -> String {
    dirs_next_home().unwrap_or_else(|| "/".into())
}

fn dirs_next_home() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("USERPROFILE").ok()
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME").ok()
    }
}
