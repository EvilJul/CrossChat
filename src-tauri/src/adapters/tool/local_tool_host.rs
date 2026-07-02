use async_trait::async_trait;
use std::process::Command;
use std::time::{Duration, Instant};
use crate::core::models::{ToolDefinition, ToolCall, ToolResult as ToolResultModel};
use crate::ports::tool_host::{ToolHost, ToolError};
use super::sandbox;

pub struct LocalToolHost {
    work_dir: String,
}

impl LocalToolHost {
    pub fn new(work_dir: String) -> Self {
        Self { work_dir }
    }

    pub fn with_default() -> Self {
        Self {
            work_dir: std::env::current_dir()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        }
    }

    fn resolve_path(&self, path: &str) -> std::path::PathBuf {
        let p = std::path::PathBuf::from(path);
        if p.is_absolute() { p }
        else { std::path::PathBuf::from(&self.work_dir).join(path) }
    }

    fn read_file(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolve_path(path);
        if !sandbox::is_path_allowed(&resolved) {
            return Err(format!("安全策略拒绝访问: {}", resolved.display()));
        }
        std::fs::read_to_string(&resolved)
            .map_err(|e| format!("无法读取文件 {}: {}", path, e))
    }

    fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        let resolved = self.resolve_path(path);
        if !sandbox::is_path_allowed(&resolved) {
            return Err(format!("安全策略拒绝访问: {}", resolved.display()));
        }
        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("无法创建目录: {}", e))?;
            }
        }
        std::fs::write(&resolved, content)
            .map_err(|e| format!("无法写入文件 {}: {}", path, e))
    }

    fn delete_file(&self, path: &str) -> Result<(), String> {
        let resolved = self.resolve_path(path);
        if !sandbox::is_path_allowed(&resolved) {
            return Err(format!("安全策略拒绝访问: {}", resolved.display()));
        }
        if !resolved.exists() {
            return Err(format!("文件不存在: {}", path));
        }
        std::fs::remove_file(&resolved)
            .map_err(|e| format!("无法删除文件 {}: {}", path, e))
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        let resolved = self.resolve_path(path);
        if !sandbox::is_path_allowed(&resolved) {
            return Err(format!("安全策略拒绝访问: {}", resolved.display()));
        }
        if !resolved.exists() {
            return Err(format!("目录不存在: {}", path));
        }
        let mut entries: Vec<String> = std::fs::read_dir(&resolved)
            .map_err(|e| format!("无法读取目录 {}: {}", path, e))?
            .flatten()
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or(false);
                format!("{} {}", if is_dir { "[目录]" } else { "[文件]" }, name)
            })
            .collect();
        entries.sort();
        Ok(entries)
    }

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
        ];
        for (pattern, desc) in patterns {
            if lower.contains(pattern) {
                return Some(desc);
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    fn new_command(program: &str) -> Command {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new(program);
        cmd.creation_flags(0x08000000);
        cmd
    }

    #[cfg(not(target_os = "windows"))]
    fn new_command(program: &str) -> Command {
        Command::new(program)
    }

    async fn run_command(&self, command: &str, cwd: &str) -> Result<(bool, String), String> {
        if let Some(danger) = Self::detect_dangerous(command) {
            return Err(format!("危险命令已阻止 ({}): {}", danger, command));
        }

        let work_dir = if !cwd.is_empty() { cwd.to_string() } else { self.work_dir.clone() };
        let resolved = std::path::PathBuf::from(&work_dir);
        if !sandbox::is_path_allowed(&resolved) {
            return Err(format!("安全策略拒绝在该目录执行命令: {}", work_dir));
        }

        let cmd_owned = command.to_string();
        let work_dir_owned = work_dir.clone();

        let result = tokio::time::timeout(
            Duration::from_secs(30),
            tokio::task::spawn_blocking(move || {
                let mut cmd = if cfg!(target_os = "windows") {
                    Self::new_command("cmd")
                } else {
                    Self::new_command("sh")
                };
                cmd.args(["/C", &cmd_owned]);
                cmd.current_dir(&work_dir_owned);
                let output = cmd.output()
                    .map_err(|e| format!("命令执行失败: {}", e))?;

                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let mut result = String::new();
                if !stdout.trim().is_empty() {
                    result.push_str(&format!("输出:\n{}", stdout));
                }
                if !stderr.trim().is_empty() {
                    if !result.is_empty() { result.push('\n'); }
                    result.push_str(&format!("错误输出:\n{}", stderr));
                }
                if result.is_empty() {
                    result = "命令执行完成（无输出）".into();
                }
                if result.len() > 10000 {
                    result.truncate(10000);
                    result.push_str("\n...（输出已截断）");
                }
                Ok((output.status.success(), result))
            }),
        ).await;

        match result {
            Ok(Ok(Ok((ok, out)))) => Ok((ok, out)),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(join_err)) => Err(join_err.to_string()),
            Err(_) => Err("命令执行超时 (30秒)".into()),
        }
    }
}

#[async_trait]
impl ToolHost for LocalToolHost {
    async fn execute(&self, call: &ToolCall) -> Result<ToolResultModel, ToolError> {
        let start = Instant::now();
        let result = match call.name.as_str() {
            "read_file" => {
                let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
                match self.read_file(path) {
                    Ok(c) => (true, c),
                    Err(e) => (false, e),
                }
            }
            "write_file" => {
                let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
                let content = call.arguments.get("content").and_then(|v| v.as_str()).unwrap_or("");
                match self.write_file(path, content) {
                    Ok(_) => (true, format!("文件已创建/更新: {}", path)),
                    Err(e) => (false, e),
                }
            }
            "list_directory" => {
                let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
                match self.list_dir(path) {
                    Ok(entries) => (true, format!("目录 {} 的内容:\n{}", path, entries.join("\n"))),
                    Err(e) => (false, e),
                }
            }
            "delete_file" => {
                let path = call.arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
                match self.delete_file(path) {
                    Ok(_) => (true, format!("文件已删除: {}", path)),
                    Err(e) => (false, e),
                }
            }
            "run_command" => {
                let command = call.arguments.get("command").and_then(|v| v.as_str()).unwrap_or("");
                let cwd = call.arguments.get("cwd").and_then(|v| v.as_str()).unwrap_or("");
                match self.run_command(command, cwd).await {
                    Ok((_success, content)) => (true, content),
                    Err(e) => (false, e),
                }
            }
            _ => return Err(ToolError::NotFound(call.name.clone())),
        };

        let elapsed = start.elapsed().as_millis() as u64;
        Ok(if result.0 {
            ToolResultModel::success(call.id.clone(), result.1, elapsed)
        } else {
            ToolResultModel::error(call.id.clone(), result.1, elapsed)
        })
    }

    async fn list_tools(&self) -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "read_file".into(),
                description: "读取文件内容".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "文件路径"}
                    },
                    "required": ["path"]
                }),
            },
            ToolDefinition {
                name: "write_file".into(),
                description: "创建或覆写文件".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "文件路径"},
                        "content": {"type": "string", "description": "文件内容"}
                    },
                    "required": ["path", "content"]
                }),
            },
            ToolDefinition {
                name: "list_directory".into(),
                description: "列出目录内容".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "目录路径"}
                    },
                    "required": ["path"]
                }),
            },
            ToolDefinition {
                name: "delete_file".into(),
                description: "删除文件".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "文件路径"}
                    },
                    "required": ["path"]
                }),
            },
            ToolDefinition {
                name: "run_command".into(),
                description: "执行命令行命令".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {"type": "string", "description": "要执行的命令"},
                        "cwd": {"type": "string", "description": "工作目录（可选）"}
                    },
                    "required": ["command"]
                }),
            },
        ]
    }

    fn is_tool_allowed(&self, name: &str) -> bool {
        matches!(name, "read_file" | "write_file" | "list_directory" | "delete_file" | "run_command")
    }
}
