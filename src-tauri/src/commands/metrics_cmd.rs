use crate::metrics::global_metrics;
use tauri::command;

/// 获取工具统计
#[command]
pub async fn get_tool_stats(tool_name: Option<String>) -> Result<serde_json::Value, String> {
    if let Some(name) = tool_name {
        global_metrics()
            .get_stats(&name)
            .map(|stats| serde_json::to_value(stats).unwrap_or_default())
            .map_err(|e| format!("获取统计失败: {}", e))
    } else {
        global_metrics()
            .get_all_stats()
            .map(|stats| serde_json::to_value(stats).unwrap_or_default())
            .map_err(|e| format!("获取统计失败: {}", e))
    }
}

/// 清理旧指标数据
#[command]
pub async fn cleanup_metrics(keep_days: i64) -> Result<usize, String> {
    global_metrics()
        .cleanup(keep_days)
        .map_err(|e| format!("清理失败: {}", e))
}
