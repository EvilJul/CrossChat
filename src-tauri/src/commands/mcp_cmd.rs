use crate::mcp::{global_mcp, McpServerConfig};

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
