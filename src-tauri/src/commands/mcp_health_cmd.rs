use crate::mcp::health::global_mcp_health;
use crate::mcp::global_mcp;
use tauri::command;

/// 检查 MCP 服务器健康
#[command]
pub async fn check_mcp_health(server_id: String) -> Result<serde_json::Value, String> {
    let servers = global_mcp().list_servers().await;
    let server = servers
        .iter()
        .find(|s| s.id == server_id)
        .ok_or("服务器未找到")?;

    let health = global_mcp_health()
        .check_server(&server_id, &server.command, &server.args)
        .await;

    let _ = global_mcp_health().record(&health);

    serde_json::to_value(&health).map_err(|e| e.to_string())
}

/// 获取所有 MCP 服务器健康状态
#[command]
pub async fn get_all_mcp_health() -> Result<Vec<serde_json::Value>, String> {
    let servers = global_mcp().list_servers().await;
    let mut results = Vec::new();

    for server in servers {
        if let Ok(Some(health)) = global_mcp_health().get_health(&server.id) {
            if let Ok(value) = serde_json::to_value(&health) {
                results.push(value);
            }
        }
    }

    Ok(results)
}
