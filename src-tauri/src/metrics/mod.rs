use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 工具性能记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    pub tool_name: String,
    pub execution_time_ms: i64,
    pub success: bool,
    pub timestamp: i64,
}

/// 工具统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStats {
    pub tool_name: String,
    pub total_calls: i64,
    pub success_calls: i64,
    pub avg_time_ms: f64,
    pub max_time_ms: i64,
    pub min_time_ms: i64,
}

/// 性能监控管理器
pub struct MetricsManager {
    db_path: PathBuf,
}

impl MetricsManager {
    pub fn new() -> Result<Self> {
        let home = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(&home).join(".crosschat");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("metrics.db");

        let mgr = Self { db_path };
        mgr.init_db()?;
        Ok(mgr)
    }

    fn init_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS tool_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tool_name TEXT NOT NULL,
                execution_time_ms INTEGER NOT NULL,
                success INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tool_name ON tool_metrics(tool_name)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON tool_metrics(timestamp DESC)",
            [],
        )?;
        Ok(())
    }

    /// 记录工具执行
    pub fn record(&self, metrics: &ToolMetrics) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO tool_metrics (tool_name, execution_time_ms, success, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                metrics.tool_name,
                metrics.execution_time_ms,
                metrics.success as i32,
                metrics.timestamp,
            ],
        )?;
        Ok(())
    }

    /// 获取工具统计
    pub fn get_stats(&self, tool_name: &str) -> Result<Option<ToolStats>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT
                COUNT(*) as total_calls,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_calls,
                AVG(execution_time_ms) as avg_time,
                MAX(execution_time_ms) as max_time,
                MIN(execution_time_ms) as min_time
             FROM tool_metrics
             WHERE tool_name = ?1",
        )?;

        let result = stmt.query_row(params![tool_name], |row| {
            Ok(ToolStats {
                tool_name: tool_name.to_string(),
                total_calls: row.get(0)?,
                success_calls: row.get(1)?,
                avg_time_ms: row.get(2)?,
                max_time_ms: row.get(3)?,
                min_time_ms: row.get(4)?,
            })
        });

        match result {
            Ok(stats) if stats.total_calls > 0 => Ok(Some(stats)),
            _ => Ok(None),
        }
    }

    /// 获取所有工具统计
    pub fn get_all_stats(&self) -> Result<Vec<ToolStats>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT
                tool_name,
                COUNT(*) as total_calls,
                SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_calls,
                AVG(execution_time_ms) as avg_time,
                MAX(execution_time_ms) as max_time,
                MIN(execution_time_ms) as min_time
             FROM tool_metrics
             GROUP BY tool_name
             ORDER BY total_calls DESC",
        )?;

        let stats = stmt.query_map([], |row| {
            Ok(ToolStats {
                tool_name: row.get(0)?,
                total_calls: row.get(1)?,
                success_calls: row.get(2)?,
                avg_time_ms: row.get(3)?,
                max_time_ms: row.get(4)?,
                min_time_ms: row.get(5)?,
            })
        })?;

        stats.collect()
    }

    /// 清理旧数据（保留最近 N 天）
    pub fn cleanup(&self, keep_days: i64) -> Result<usize> {
        let conn = Connection::open(&self.db_path)?;
        let cutoff = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - (keep_days * 86400);

        let deleted = conn.execute(
            "DELETE FROM tool_metrics WHERE timestamp < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }
}

impl Default for MetricsManager {
    fn default() -> Self {
        Self::new().expect("无法初始化性能监控管理器")
    }
}

/// 全局性能监控管理器
static METRICS_MANAGER: std::sync::LazyLock<MetricsManager> =
    std::sync::LazyLock::new(|| MetricsManager::new().expect("无法初始化性能监控管理器"));

pub fn global_metrics() -> &'static MetricsManager {
    &METRICS_MANAGER
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

