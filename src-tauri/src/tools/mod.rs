pub mod file_tools;
pub mod shell_tool;

use serde::{Deserialize, Serialize};
use crate::providers::types::ToolDefinition;

/// 工具执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
}

/// 获取所有可用工具定义 (OpenAI 兼容 JSON Schema)
pub fn get_all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "read_file".into(),
            description: "读取文件内容。参数 path 为文件路径（绝对路径或相对于工作目录）。".into(),
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
            description: "创建或覆写文件。参数 path 为文件路径，content 为文件内容。".into(),
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
            name: "delete_file".into(),
            description: "删除文件（需要用户确认）。参数 path 为文件路径。".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "文件路径"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "list_directory".into(),
            description: "列出目录下的文件和子目录。参数 path 为目录路径。".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "目录路径"}
                },
                "required": ["path"]
            }),
        },
        ToolDefinition {
            name: "run_command".into(),
            description: "执行命令行命令（需要用户确认）。参数 command 为要执行的命令，cwd 为可选的工作目录。".into(),
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

/// 执行工具调用
pub async fn execute_tool(name: &str, arguments: &serde_json::Value, work_dir: &str) -> ToolResult {
    match name {
        "read_file" => {
            let path = arguments["path"].as_str().unwrap_or("");
            file_tools::read_file(path, work_dir)
        }
        "write_file" => {
            let path = arguments["path"].as_str().unwrap_or("");
            let content = arguments["content"].as_str().unwrap_or("");
            file_tools::write_file(path, content, work_dir)
        }
        "delete_file" => {
            let path = arguments["path"].as_str().unwrap_or("");
            file_tools::delete_file(path, work_dir)
        }
        "list_directory" => {
            let path = arguments["path"].as_str().unwrap_or("");
            file_tools::list_dir(path, work_dir)
        }
        "run_command" => {
            let command = arguments["command"].as_str().unwrap_or("");
            let cwd = arguments["cwd"].as_str().unwrap_or("");
            shell_tool::run_command(command, cwd, work_dir).await
        }
        _ => ToolResult {
            success: false,
            content: format!("未知工具: {}", name),
        },
    }
}
