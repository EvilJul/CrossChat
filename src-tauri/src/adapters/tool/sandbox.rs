use std::path::{Path, PathBuf};

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

pub fn is_path_allowed(path: &Path) -> bool {
    let canonical = if let Ok(c) = path.canonicalize() {
        c
    } else {
        let mut clean = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::ParentDir => { clean.pop(); }
                std::path::Component::Normal(c) => { clean.push(c); }
                _ => {}
            }
        }
        clean
    };

    let path_str = canonical.to_string_lossy().to_string();
    for prefix in BLOCKED_PREFIXES {
        if path_str.starts_with(prefix) {
            return false;
        }
    }

    let sensitive_dirs = [".ssh", ".gnupg", ".aws", ".config/gcloud"];
    for sensitive in &sensitive_dirs {
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
