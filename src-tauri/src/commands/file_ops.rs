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

/// 读取文件内容（用于预览面板）
#[tauri::command]
pub async fn read_file_content(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if file_path.is_dir() {
        return Err(format!("路径是目录，不是文件: {}", path));
    }
    // 限制预览文件大小为 1MB
    let metadata = file_path.metadata().map_err(|e| format!("读取元数据失败: {}", e))?;
    if metadata.len() > 1_048_576 {
        return Err("文件过大（超过 1MB），无法预览".to_string());
    }
    std::fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {}", e))
}

/// 删除文件或空目录（用于工作区右键菜单）
#[tauri::command]
pub async fn delete_file_or_dir(path: String) -> Result<String, String> {
    let p = PathBuf::from(&path);
    if !p.exists() { return Err("文件不存在".into()); }
    if p.is_dir() {
        std::fs::remove_dir_all(&p).map_err(|e| format!("删除目录失败: {}", e))?;
    } else {
        std::fs::remove_file(&p).map_err(|e| format!("删除文件失败: {}", e))?;
    }
    let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    Ok(format!("已删除: {}", name))
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
