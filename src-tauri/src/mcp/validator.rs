use std::time::Duration;
use serde::{Deserialize, Serialize};

/// MCP 验证结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub success: bool,
    pub message: String,
    pub details: Option<ValidationDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetails {
    pub command_exists: bool,
    pub command_version: Option<String>,
    pub tools_discovered: Option<Vec<String>>,
    pub response_time_ms: i64,
}

/// 验证命令是否存在
pub fn validate_command_exists(command: &str) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    let (shell, shell_arg) = ("cmd", "/c");
    #[cfg(not(target_os = "windows"))]
    let (shell, shell_arg) = ("sh", "-c");

    #[cfg(target_os = "windows")]
    let check_cmd = format!("where {}", command);
    #[cfg(not(target_os = "windows"))]
    let check_cmd = format!("which {}", command);

    let output = std::process::Command::new(shell)
        .args(&[shell_arg, &check_cmd])
        .output()
        .map_err(|e| format!("无法检查命令: {}", e))?;

    if !output.status.success() {
        return Err(format_command_not_found_error(command));
    }

    // 尝试获取版本信息
    let version = get_command_version(command).unwrap_or_else(|_| "未知版本".to_string());
    Ok(version)
}

/// 获取命令版本
fn get_command_version(command: &str) -> Result<String, String> {
    let version_args = match command {
        "uvx" | "uv" => vec!["--version"],
        "npx" => vec!["--version"],
        "node" => vec!["--version"],
        "python" | "python3" => vec!["--version"],
        _ => vec!["--version"],
    };

    #[cfg(target_os = "windows")]
    let (shell, shell_arg) = ("cmd", "/c");
    #[cfg(not(target_os = "windows"))]
    let (shell, shell_arg) = ("sh", "-c");

    let cmd_str = format!("{} {}", command, version_args.join(" "));

    let output = std::process::Command::new(shell)
        .args(&[shell_arg, &cmd_str])
        .output()
        .map_err(|e| format!("无法获取版本: {}", e))?;

    if output.status.success() {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            return Ok(version);
        }
        // 有些命令版本信息在 stderr
        let version = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !version.is_empty() {
            return Ok(version);
        }
    }

    Err("无法获取版本信息".to_string())
}

/// 格式化"命令未找到"错误信息
fn format_command_not_found_error(command: &str) -> String {
    let install_guide = match command {
        "uvx" | "uv" => {
            r#"
❌ 命令 'uvx' 未找到

uv/uvx 是 Python 包管理工具，用于运行 Python MCP 服务器。

📦 安装方法：

Windows (PowerShell):
  powershell -c "irm https://astral.sh/uv/install.ps1 | iex"

macOS/Linux:
  curl -LsSf https://astral.sh/uv/install.sh | sh

✅ 验证安装：
  在终端运行: uvx --version

📚 详细文档：https://docs.astral.sh/uv/
"#
        }
        "npx" => {
            r#"
❌ 命令 'npx' 未找到

npx 是 Node.js 包运行工具，用于运行 JavaScript MCP 服务器。

📦 安装方法：

1. 下载并安装 Node.js:
   https://nodejs.org/

2. 选择 LTS（长期支持）版本

3. 安装完成后，npx 会自动包含在内

✅ 验证安装：
  在终端运行: npx --version

📚 详细文档：https://nodejs.org/
"#
        }
        "node" => {
            r#"
❌ 命令 'node' 未找到

Node.js 是 JavaScript 运行环境，用于运行 JavaScript MCP 服务器。

📦 安装方法：

下载并安装 Node.js:
  https://nodejs.org/

选择 LTS（长期支持）版本

✅ 验证安装：
  在终端运行: node --version

📚 详细文档：https://nodejs.org/
"#
        }
        "python" | "python3" => {
            r#"
❌ 命令 'python' 未找到

Python 是编程语言，用于运行 Python MCP 服务器。

📦 安装方法：

Windows:
  https://www.python.org/downloads/

macOS:
  brew install python3

Linux:
  sudo apt install python3  # Ubuntu/Debian
  sudo yum install python3  # CentOS/RHEL

✅ 验证安装：
  在终端运行: python --version

📚 详细文档：https://www.python.org/
"#
        }
        _ => {
            &format!(
                r#"
❌ 命令 '{}' 未找到

请确保已安装相关依赖，并将其添加到系统 PATH 环境变量。

💡 提示：
1. 检查命令是否正确拼写
2. 确认已安装相关软件
3. 重启应用以刷新环境变量
4. 在终端运行命令验证是否可用
"#,
                command
            )
        }
    };

    install_guide.to_string()
}

/// 完整验证 MCP 服务器
pub async fn validate_mcp_server(
    command: String,
    args: Vec<String>,
) -> Result<ValidationResult, String> {
    let start = std::time::Instant::now();

    // 1. 检查命令是否存在
    let command_version = match validate_command_exists(&command) {
        Ok(version) => version,
        Err(e) => {
            return Ok(ValidationResult {
                success: false,
                message: e,
                details: Some(ValidationDetails {
                    command_exists: false,
                    command_version: None,
                    tools_discovered: None,
                    response_time_ms: start.elapsed().as_millis() as i64,
                }),
            });
        }
    };

    // 2. 尝试发现工具（超时 15 秒）
    let tools_result = tokio::time::timeout(
        Duration::from_secs(15),
        crate::mcp::server::discover_tools(command.clone(), args.clone()),
    )
    .await;

    let response_time_ms = start.elapsed().as_millis() as i64;

    match tools_result {
        Ok(Ok(tools)) => {
            let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();
            let tool_count = tool_names.len();

            Ok(ValidationResult {
                success: true,
                message: format!(
                    "✅ MCP 服务器验证成功！\n\n\
                     命令: {}\n\
                     版本: {}\n\
                     发现工具: {} 个\n\
                     响应时间: {} ms",
                    command, command_version, tool_count, response_time_ms
                ),
                details: Some(ValidationDetails {
                    command_exists: true,
                    command_version: Some(command_version),
                    tools_discovered: Some(tool_names),
                    response_time_ms,
                }),
            })
        }
        Ok(Err(e)) => Ok(ValidationResult {
            success: false,
            message: format!(
                "❌ MCP 服务器启动失败\n\n\
                 命令: {}\n\
                 版本: {}\n\
                 错误: {}\n\n\
                 💡 可能的原因：\n\
                 1. MCP 服务器包未安装或不存在\n\
                 2. 网络连接问题（无法下载包）\n\
                 3. 参数配置错误\n\
                 4. 服务器代码有错误\n\n\
                 🔧 建议：\n\
                 1. 检查包名是否正确\n\
                 2. 确认网络连接正常\n\
                 3. 在终端手动运行命令测试：\n\
                    {} {}",
                command,
                command_version,
                e,
                command,
                args.join(" ")
            ),
            details: Some(ValidationDetails {
                command_exists: true,
                command_version: Some(command_version),
                tools_discovered: None,
                response_time_ms,
            }),
        }),
        Err(_) => Ok(ValidationResult {
            success: false,
            message: format!(
                "❌ MCP 服务器启动超时（15 秒）\n\n\
                 命令: {}\n\
                 版本: {}\n\n\
                 💡 可能的原因：\n\
                 1. 首次运行需要下载依赖（可能需要更长时间）\n\
                 2. 网络速度慢\n\
                 3. 服务器启动过程卡住\n\n\
                 🔧 建议：\n\
                 1. 在终端手动运行命令，观察输出：\n\
                    {} {}\n\
                 2. 等待下载完成后再添加\n\
                 3. 检查网络连接",
                command, command_version, command, args.join(" ")
            ),
            details: Some(ValidationDetails {
                command_exists: true,
                command_version: Some(command_version),
                tools_discovered: None,
                response_time_ms,
            }),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_command_exists() {
        // 测试系统命令（应该存在）
        #[cfg(target_os = "windows")]
        let result = validate_command_exists("cmd");
        #[cfg(not(target_os = "windows"))]
        let result = validate_command_exists("sh");

        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_nonexistent_command() {
        let result = validate_command_exists("this_command_definitely_does_not_exist_12345");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("未找到"));
    }

    #[tokio::test]
    async fn test_validate_invalid_mcp_server() {
        let result = validate_mcp_server(
            "echo".to_string(),
            vec!["invalid".to_string()],
        )
        .await;

        assert!(result.is_ok());
        let validation = result.unwrap();
        assert!(!validation.success);
    }
}

