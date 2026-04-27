use std::path::{Path, PathBuf};

/// 关键系统路径黑名单
const BLOCKED_PREFIXES: &[&str] = &[
    "/etc/",
    "/boot/",
    "/sys/",
    "/proc/",
    "/dev/",
    "C:\\Windows\\System32\\config",
    "C:\\Windows\\System32\\drivers",
    "C:\\Program Files\\WindowsApps",
];

/// 检查路径是否在允许范围内
pub fn is_path_allowed(path: &Path) -> bool {
    // 规范化路径
    let canonical = if let Ok(c) = path.canonicalize() {
        c
    } else {
        // 路径不存在时，用逻辑规范化
        let mut clean = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => {
                    clean.pop();
                }
                std::path::Component::Normal(c) => {
                    clean.push(c);
                }
                _ => {}
            }
        }
        clean
    };

    let path_str = canonical.to_string_lossy().to_string();

    // 检查黑名单
    for prefix in BLOCKED_PREFIXES {
        if path_str.starts_with(prefix) {
            return false;
        }
    }

    // 检查 .ssh, .gnupg 等敏感目录
    let sensitive_dirs = [".ssh", ".gnupg", ".aws", ".config/gcloud"];
    for sensitive in &sensitive_dirs {
        // 路径中包含敏感目录的完整路径段
        let pattern = if cfg!(target_os = "windows") {
            format!("\\{}\\", sensitive.replace('/', "\\"))
        } else {
            format!("/{}/", sensitive)
        };
        if path_str.contains(&pattern) {
            return false;
        }
    }

    true
}
