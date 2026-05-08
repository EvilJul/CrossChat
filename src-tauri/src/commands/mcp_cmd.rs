use crate::mcp::{global_mcp, McpServerConfig};

/// 验证 MCP 命令是否可用
#[tauri::command]
pub async fn validate_mcp_command(command: String) -> Result<String, String> {
    crate::mcp::validator::validate_command_exists(&command)
}

/// 测试 MCP 服务器连接
#[tauri::command]
pub async fn test_mcp_server(
    command: String,
    args: Vec<String>,
) -> Result<serde_json::Value, String> {
    let result = crate::mcp::validator::validate_mcp_server(command, args).await?;
    serde_json::to_value(&result).map_err(|e| e.to_string())
}

/// 添加 MCP 服务器
#[tauri::command]
pub async fn add_mcp_server(config: McpServerConfig) -> Result<(), String> {
    global_mcp().add_server(config).await;
    Ok(())
}

/// 删除 MCP 服务器
#[tauri::command]
pub async fn remove_mcp_server(id: String) -> Result<(), String> {
    global_mcp().remove_server(&id).await;
    Ok(())
}

/// 启用/禁用 MCP 服务器
#[tauri::command]
pub async fn toggle_mcp_server(id: String, enabled: bool) -> Result<(), String> {
    global_mcp().set_enabled(&id, enabled).await;
    Ok(())
}

/// 列出所有 MCP 服务器
#[tauri::command]
pub async fn list_mcp_servers() -> Result<Vec<McpServerConfig>, String> {
    Ok(global_mcp().list_servers().await)
}

/// 刷新 MCP 工具列表（清缓存重发现）
#[tauri::command]
pub async fn refresh_mcp_tools() -> Result<usize, String> {
    let tools = global_mcp().get_all_tools().await;
    Ok(tools.len())
}
