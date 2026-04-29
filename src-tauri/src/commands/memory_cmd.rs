use crate::memory::global_memory;
use tauri::command;

/// 获取最近的记忆
#[command]
pub async fn get_recent_memories(limit: usize) -> Result<Vec<crate::memory::Memory>, String> {
    global_memory()
        .get_recent(limit)
        .map_err(|e| format!("获取记忆失败: {}", e))
}

/// 搜索记忆
#[command]
pub async fn search_memories(query: String, limit: usize) -> Result<Vec<crate::memory::Memory>, String> {
    global_memory()
        .search(&query, limit)
        .map_err(|e| format!("搜索记忆失败: {}", e))
}

/// 清理旧记忆
#[command]
pub async fn cleanup_memories(keep_count: usize) -> Result<usize, String> {
    global_memory()
        .cleanup(keep_count)
        .map_err(|e| format!("清理记忆失败: {}", e))
}
