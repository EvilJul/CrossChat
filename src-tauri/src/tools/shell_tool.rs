use super::ToolResult;
use crate::security::sandbox;
use std::process::Command;
use std::time::Duration;

/// 危险命令模式检测（简单子字符串匹配）
fn detect_dangerous(command: &str) -> Option<&'static str> {
    let lower = command.to_lowercase();
    let patterns: &[(&str, &str)] = &[
        ("rm -rf /", "递归删除根目录"),
        ("sudo ", "提权操作"),
        ("mkfs.", "格式化文件系统"),
        ("dd if=", "磁盘直接写入"),
        ("> /dev/", "写入设备文件"),
        ("git push --force", "强制推送"),
        ("chmod 777", "开放全部权限"),
        (":(){ :|:& };:", "fork 炸弹"),
        ("wget ", "尝试下载外部文件"),
        ("curl ", "尝试下载外部文件"),
    ];

    for (pattern, desc) in patterns {
        if lower.contains(pattern) {
            return Some(desc);
        }
    }
    None
}

pub async fn run_command(command: &str, cwd: &str, default_cwd: &str) -> ToolResult {
    // 检测危险命令
    if let Some(danger) = detect_dangerous(command) {
        return ToolResult {
            success: false,
            content: format!("危险命令已阻止 ({}): {}", danger, command),
        };
    }

    let work_dir = if !cwd.is_empty() {
        cwd.to_string()
    } else if !default_cwd.is_empty() {
        default_cwd.to_string()
    } else {
        ".".to_string()
    };

    // 检查工作目录安全
    if !sandbox::is_path_allowed(&std::path::PathBuf::from(&work_dir)) {
        return ToolResult {
            success: false,
            content: format!("安全策略拒绝在该目录执行命令: {}", work_dir),
        };
    }

    // 使用 tokio 以支持超时 (clone 以满足 'static 要求)
    let cmd_owned = command.to_string();
    let work_dir_owned = work_dir.clone();

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || {
            let output = if cfg!(target_os = "windows") {
                Command::new("cmd")
                    .args(["/C", &cmd_owned])
                    .current_dir(&work_dir_owned)
                    .output()
            } else {
                Command::new("sh")
                    .args(["-c", &cmd_owned])
                    .current_dir(&work_dir_owned)
                    .output()
            };

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut result = String::new();

                    if !stdout.trim().is_empty() {
                        result.push_str(&format!("输出:\n{}", stdout));
                    }
                    if !stderr.trim().is_empty() {
                        if !result.is_empty() {
                            result.push('\n');
                        }
                        result.push_str(&format!("错误输出:\n{}", stderr));
                    }
                    if result.is_empty() {
                        result = "命令执行完成（无输出）".into();
                    }

                    // 截断过长输出
                    if result.len() > 10000 {
                        result.truncate(10000);
                        result.push_str("\n...（输出已截断）");
                    }

                    ToolResult {
                        success: out.status.success(),
                        content: result,
                    }
                }
                Err(e) => ToolResult {
                    success: false,
                    content: format!("命令执行失败: {}", e),
                },
            }
        }),
    )
    .await;

    match result {
        Ok(Ok(tool_result)) => tool_result,
        Ok(Err(e)) => ToolResult {
            success: false,
            content: format!("命令执行异常: {}", e),
        },
        Err(_) => ToolResult {
            success: false,
            content: "命令执行超时 (30秒)".into(),
        },
    }
}
