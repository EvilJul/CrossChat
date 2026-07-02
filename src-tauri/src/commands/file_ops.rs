use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

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

// ============================================================================
// 敏感路径黑名单校验
//
// 所有文件操作 command 在实际访问文件系统前，必须先经过 `is_path_forbidden`
// 校验，防止读/写/删用户凭据（~/.ssh、~/.aws 等）与应用自身数据（~/.crosschat）。
// 防绕过策略见各辅助函数注释。
// ============================================================================

/// 构建黑名单目录列表（已尽量规范化）。
///
/// - 以 `dirs::home_dir()` 拼接用户级敏感目录；家目录不可用时跳过这部分。
/// - 追加 Unix 系统级敏感目录。
/// - 每个目录都尝试 `canonicalize`：既能处理 macOS 上 `/var -> /private/var`
///   之类软链，也能保证与被检查路径的规范化结果在同一命名空间下比较；
///   目录不存在（canonicalize 失败）时退回其组件归一化形式，仍保留在黑名单里。
fn blacklist_dirs() -> Vec<PathBuf> {
    let mut raw: Vec<PathBuf> = Vec::new();

    // 用户级敏感目录（凭据 / 密钥 / 云配置 + 应用自身数据）
    if let Some(home) = dirs::home_dir() {
        for sub in [
            ".ssh",
            ".aws",
            ".gnupg",
            ".config/gcloud",
            ".kube",
            ".docker",
            ".netrc",
            ".crosschat",
        ] {
            raw.push(home.join(sub));
        }
    }

    // Unix 系统级敏感目录
    for sys in ["/etc", "/root", "/var/root"] {
        raw.push(PathBuf::from(sys));
    }

    // 规范化：存在则 canonicalize（解析软链），否则退回组件归一化
    raw.into_iter()
        .map(|p| std::fs::canonicalize(&p).unwrap_or_else(|_| normalize_components(&p)))
        .collect()
}

/// 基于路径组件做 `..` / `.` 归一化的兜底实现（不触碰文件系统）。
///
/// 当路径或其祖先都不存在、无法 canonicalize 时使用，用于挡住纯字符串层面的
/// `../` 穿越。注意：这只是兜底，无法解析软链，真正的软链防护依赖 canonicalize。
fn normalize_components(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            // 前缀（Windows 盘符）与根目录：直接保留
            Component::Prefix(_) | Component::RootDir => out.push(comp.as_os_str()),
            // "." 忽略
            Component::CurDir => {}
            // ".." 回退一级；已到根或无可回退时忽略，避免越过根
            Component::ParentDir => {
                if !out.pop() {
                    // 相对路径场景下保留 ".."，尽量不丢信息
                    if !out.has_root() {
                        out.push("..");
                    }
                }
            }
            Component::Normal(seg) => out.push(seg),
        }
    }
    out
}

/// 将被检查路径解析为可与黑名单比较的规范化绝对路径。
///
/// - 路径本身存在：直接 `canonicalize`（解析软链 + `..`）。
/// - 路径不存在（write / delete 目标可能尚未创建）：向上寻找**最近存在的祖先**
///   并对其 canonicalize，再把剩余的、经组件归一化的尾部拼接回去；这样即便
///   目标文件不存在，也能识别出它落在软链解析后的敏感目录内。
/// - 完全无法解析时：退回对原始路径做组件归一化，作为最后兜底。
fn resolve_check_path(path: &Path) -> PathBuf {
    // 情况一：路径存在，直接规范化
    if let Ok(canon) = std::fs::canonicalize(path) {
        return canon;
    }

    // 情况二：向上找最近存在的祖先，规范化后再拼回剩余尾部
    let mut ancestor = path;
    while let Some(parent) = ancestor.parent() {
        if let Ok(canon_parent) = std::fs::canonicalize(parent) {
            // 剩余尾部 = 原始路径相对该 parent 的部分
            if let Ok(rest) = path.strip_prefix(parent) {
                // 对拼接结果整体再做一次组件归一化，处理 rest 中可能出现的 ".."
                return normalize_components(&canon_parent.join(rest));
            }
            return canon_parent;
        }
        ancestor = parent;
    }

    // 情况三：兜底——纯组件归一化
    normalize_components(path)
}

/// 判断给定路径是否落入敏感目录黑名单（命中即禁止访问）。
///
/// 命中规则：被检查路径规范化后，等于某个黑名单目录，或位于其子树之下。
/// 通过 `canonicalize` + 组件归一化双重手段，防止 `../` 穿越与软链绕过。
fn is_path_forbidden(path: &Path) -> bool {
    let target = resolve_check_path(path);
    for banned in blacklist_dirs() {
        // 相等或以黑名单目录为前缀（即在其子树内）均视为命中
        if target == banned || target.starts_with(&banned) {
            return true;
        }
    }
    false
}

/// 对外统一的校验入口：命中黑名单时返回中文错误。
fn ensure_path_allowed(path: &Path) -> Result<(), String> {
    if is_path_forbidden(path) {
        return Err("拒绝访问受保护路径".to_string());
    }
    Ok(())
}

#[tauri::command]
pub async fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = PathBuf::from(&path);
    ensure_path_allowed(&dir)?;
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
    ensure_path_allowed(&file_path)?;
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
    ensure_path_allowed(&file_path)?;
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
    ensure_path_allowed(&file_path)?;
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
    ensure_path_allowed(&file_path)?;
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("创建父目录失败: {}", e))?;
    }
    std::fs::write(&file_path, bytes).map_err(|e| format!("写入文件失败: {}", e))
}

#[tauri::command]
pub async fn delete_file_or_dir(path: String) -> Result<String, String> {
    let p = PathBuf::from(&path);
    ensure_path_allowed(&p)?;
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
