use super::ToolResult;
use crate::security::sandbox;
use std::path::PathBuf;

fn resolve_path(path: &str, work_dir: &str) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else if !work_dir.is_empty() {
        PathBuf::from(work_dir).join(path)
    } else {
        p
    }
}

pub fn read_file(path: &str, work_dir: &str) -> ToolResult {
    let resolved = resolve_path(path, work_dir);
    if !sandbox::is_path_allowed(&resolved) {
        return ToolResult {
            success: false,
            content: format!("安全策略拒绝访问: {}", resolved.display()),
        };
    }
    match std::fs::read_to_string(&resolved) {
        Ok(content) => ToolResult {
            success: true,
            content: format!("文件内容:\n{}", content),
        },
        Err(e) => ToolResult {
            success: false,
            content: format!("无法读取文件 {}: {}", resolved.display(), e),
        },
    }
}

pub fn write_file(path: &str, content: &str, work_dir: &str) -> ToolResult {
    let resolved = resolve_path(path, work_dir);
    if !sandbox::is_path_allowed(&resolved) {
        return ToolResult {
            success: false,
            content: format!("安全策略拒绝访问: {}", resolved.display()),
        };
    }
    // 确保父目录存在
    if let Some(parent) = resolved.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolResult {
                    success: false,
                    content: format!("无法创建目录 {}: {}", parent.display(), e),
                };
            }
        }
    }
    match std::fs::write(&resolved, content) {
        Ok(_) => ToolResult {
            success: true,
            content: format!("文件已创建/更新: {}", resolved.display()),
        },
        Err(e) => ToolResult {
            success: false,
            content: format!("无法写入文件 {}: {}", resolved.display(), e),
        },
    }
}

pub fn delete_file(path: &str, work_dir: &str) -> ToolResult {
    let resolved = resolve_path(path, work_dir);
    if !sandbox::is_path_allowed(&resolved) {
        return ToolResult {
            success: false,
            content: format!("安全策略拒绝访问: {}", resolved.display()),
        };
    }
    if !resolved.exists() {
        return ToolResult {
            success: false,
            content: format!("文件不存在: {}", resolved.display()),
        };
    }
    match std::fs::remove_file(&resolved) {
        Ok(_) => ToolResult {
            success: true,
            content: format!("文件已删除: {}", resolved.display()),
        },
        Err(e) => ToolResult {
            success: false,
            content: format!("无法删除文件 {}: {}", resolved.display(), e),
        },
    }
}

pub fn list_dir(path: &str, work_dir: &str) -> ToolResult {
    let resolved = resolve_path(path, work_dir);
    if !sandbox::is_path_allowed(&resolved) {
        return ToolResult {
            success: false,
            content: format!("安全策略拒绝访问: {}", resolved.display()),
        };
    }
    if !resolved.exists() {
        return ToolResult {
            success: false,
            content: format!("目录不存在: {}", resolved.display()),
        };
    }
    match std::fs::read_dir(&resolved) {
        Ok(entries) => {
            let mut listing = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
                let prefix = if is_dir { "[目录] " } else { "[文件] " };
                listing.push(format!("{}{}", prefix, name));
            }
            listing.sort();
            ToolResult {
                success: true,
                content: format!(
                    "目录 {} 的内容:\n{}",
                    resolved.display(),
                    listing.join("\n")
                ),
            }
        }
        Err(e) => ToolResult {
            success: false,
            content: format!("无法列出目录 {}: {}", resolved.display(), e),
        },
    }
}
