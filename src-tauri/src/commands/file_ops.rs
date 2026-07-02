use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilePreviewInfo {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub is_executable: bool,
    pub file_type: String,
    pub preview_content: Option<String>,
}

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

    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
    Ok(entries)
}

#[tauri::command]
pub async fn read_file_content(path: String) -> Result<String, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if file_path.is_dir() {
        return Err(format!("路径是目录，不是文件: {}", path));
    }
    let metadata = file_path.metadata().map_err(|e| format!("读取元数据失败: {}", e))?;
    if metadata.len() > 10_485_760 {
        return Err("文件过大（超过 10MB），无法预览".to_string());
    }
    std::fs::read_to_string(&file_path).map_err(|e| format!("读取文件失败: {}", e))
}

#[tauri::command]
pub async fn get_file_preview_info(path: String) -> Result<FilePreviewInfo, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if file_path.is_dir() {
        return Err(format!("路径是目录，不是文件: {}", path));
    }

    let metadata = file_path.metadata().map_err(|e| format!("读取元数据失败: {}", e))?;
    let name = file_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let preview_content = if metadata.len() > 10_485_760 {
        None
    } else {
        let ext = file_path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let text_exts = ["txt", "md", "json", "xml", "html", "css", "js", "ts", "rs", "py", "sh"];
        if text_exts.contains(&ext.as_str()) {
            std::fs::read_to_string(&file_path).ok()
        } else {
            Some(format!("文件: {}\n大小: {} 字节\n类型: {}", name, metadata.len(), ext))
        }
    };

    Ok(FilePreviewInfo {
        name,
        path: path.clone(),
        size: metadata.len(),
        is_executable: false,
        file_type: "unknown".into(),
        preview_content,
    })
}

#[tauri::command]
pub async fn read_file_bytes(path: String) -> Result<Vec<u8>, String> {
    let file_path = PathBuf::from(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if file_path.is_dir() {
        return Err(format!("路径是目录，不是文件: {}", path));
    }
    let metadata = file_path.metadata().map_err(|e| format!("读取元数据失败: {}", e))?;
    if metadata.len() > 52_428_800 {
        return Err("文件过大（超过 50MB）".to_string());
    }
    std::fs::read(&file_path).map_err(|e| format!("读取文件失败: {}", e))
}

#[tauri::command]
pub async fn write_file_bytes(path: String, bytes: Vec<u8>) -> Result<(), String> {
    let file_path = PathBuf::from(&path);
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {}", e))?;
    }
    std::fs::write(&file_path, bytes).map_err(|e| format!("写入文件失败: {}", e))
}

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

#[tauri::command]
pub fn get_home_dir() -> String {
    #[cfg(target_os = "windows")]
    { std::env::var("USERPROFILE").unwrap_or_default() }
    #[cfg(not(target_os = "windows"))]
    { std::env::var("HOME").unwrap_or_default() }
}
