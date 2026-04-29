use super::ToolResult;
use super::python_sandbox;
use crate::security::sandbox;
use std::process::Command;
use std::time::Duration;

/// Windows 下创建不弹出控制台窗口的 Command
#[cfg(target_os = "windows")]
fn hidden_command(program: &str) -> Command {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new(program);
    cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    cmd
}

#[cfg(not(target_os = "windows"))]
fn hidden_command(program: &str) -> Command {
    Command::new(program)
}

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

    // Python/Pip 命令走沙盒
    let is_python_cmd = {
        let first = command.split_whitespace().next().unwrap_or("").to_lowercase();
        first == "python" || first == "python3" || first.ends_with("python.exe")
            || first == "pip" || first == "pip3" || first.ends_with("pip.exe")
    };

    if is_python_cmd {
        // 确保沙盒就绪
        match python_sandbox::ensure_sandbox() {
            Ok(_) => {
                return run_python_in_sandbox(command, &work_dir).await;
            }
            Err(e) => {
                return ToolResult {
                    success: false,
                    content: format!("Python 沙盒初始化失败: {}", e),
                };
            }
        }
    }

    // 使用 tokio 以支持超时 (clone 以满足 'static 要求)
    let cmd_owned = command.to_string();
    let work_dir_owned = work_dir.clone();

    let result = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || {
            let output = if cfg!(target_os = "windows") {
                hidden_command("cmd")
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

/// 在沙盒中运行 Python 命令，自动处理模块缺失
async fn run_python_in_sandbox(command: &str, work_dir: &str) -> ToolResult {
    let sandbox_cmd = python_sandbox::sandboxify_command(command);
    let cmd_owned = sandbox_cmd.to_string();
    let orig_cmd = command.to_string();  // 保存原始命令用于重试
    let work_dir_owned = work_dir.to_string();

    // Python 脚本超时 60 秒（数据处理可能耗时较长）
    let result = tokio::time::timeout(
        Duration::from_secs(60),
        tokio::task::spawn_blocking(move || {
            let output = if cfg!(target_os = "windows") {
                hidden_command("cmd").args(["/C", &cmd_owned])
                    .current_dir(&work_dir_owned).output()
            } else {
                Command::new("sh").args(["-c", &cmd_owned])
                    .current_dir(&work_dir_owned).output()
            };

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    let mut result = String::new();
                    if !stdout.trim().is_empty() { result.push_str(&format!("输出:\n{}", stdout)); }
                    if !stderr.trim().is_empty() {
                        if !result.is_empty() { result.push('\n'); }
                        result.push_str(&format!("错误输出:\n{}", stderr));
                    }
                    if result.is_empty() { result = "命令执行完成（无输出）".into(); }
                    if result.len() > 20000 { result.truncate(20000); result.push_str("\n...（输出已截断）"); }

                    let success = out.status.success();
                    let stderr_str = String::from_utf8_lossy(&out.stderr).to_string();

                    if !success {
                        if let Some(retry_result) = detect_install_and_retry(
                            &stderr_str, &orig_cmd, &work_dir_owned
                        ) {
                            return retry_result;
                        }
                    }
                    ToolResult { success, content: result }
                }
                Err(e) => ToolResult {
                    success: false,
                    content: format!("命令执行失败: {}", e),
                },
            }
        }),
    ).await;

    match result {
        Ok(Ok(tool_result)) => tool_result,
        Ok(Err(e)) => ToolResult {
            success: false,
            content: format!("命令执行异常: {}", e),
        },
        Err(_) => ToolResult {
            success: false,
            content: "Python 命令执行超时 (60秒)".into(),
        },
    }
}

/// 检测 stderr 中的 ModuleNotFoundError，安装后重试命令
fn detect_install_and_retry(
    stderr: &str, cmd: &str, work_dir: &str
) -> Option<ToolResult> {
    let module = python_sandbox::detect_missing_module(stderr)?;
    let install_result = python_sandbox::auto_install_module(&module);

    match install_result {
        Ok(_) => {
            // 安装成功，重试原命令
            let retry_cmd = python_sandbox::sandboxify_command(cmd);
            let output = if cfg!(target_os = "windows") {
                hidden_command("cmd").args(["/C", &retry_cmd]).current_dir(work_dir).output()
            } else {
                Command::new("sh").args(["-c", &retry_cmd]).current_dir(work_dir).output()
            };

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let stderr2 = String::from_utf8_lossy(&out.stderr);
                    let mut result = format!("📦 已自动安装缺失模块 `{}`，重试命令:\n\n", module);
                    if !stdout.trim().is_empty() { result.push_str(&format!("输出:\n{}", stdout)); }
                    if !stderr2.trim().is_empty() {
                        if !result.is_empty() { result.push('\n'); }
                        result.push_str(&format!("错误输出:\n{}", stderr2));
                    }
                    if result.len() > 20000 { result.truncate(20000); result.push_str("\n...（输出已截断）"); }
                    Some(ToolResult { success: out.status.success(), content: result })
                }
                Err(e) => Some(ToolResult {
                    success: false,
                    content: format!("模块 {} 已安装，但重试命令失败: {}", module, e),
                }),
            }
        }
        Err(e) => Some(ToolResult {
            success: false,
            content: format!("模块 {} 自动安装失败: {}\n\n请手动执行: pip install {}", module, e, module),
        }),
    }
}
