use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// MCP 服务器健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHealth {
    pub server_id: String,
    pub status: HealthStatus,
    pub last_check: i64,
    pub response_time_ms: i64,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Down,
}

/// MCP 健康检查管理器
pub struct McpHealthManager {
    db_path: PathBuf,
}

impl McpHealthManager {
    pub fn new() -> Result<Self> {
        let home = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(&home).join(".crosschat");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("mcp_health.db");

        let mgr = Self { db_path };
        mgr.init_db()?;
        Ok(mgr)
    }

    fn init_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS mcp_health (
                server_id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                last_check INTEGER NOT NULL,
                response_time_ms INTEGER NOT NULL,
                error_message TEXT
            )",
            [],
        )?;
        Ok(())
    }

    /// 记录健康检查结果
    pub fn record(&self, health: &McpHealth) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT OR REPLACE INTO mcp_health (server_id, status, last_check, response_time_ms, error_message)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                health.server_id,
                format!("{:?}", health.status),
                health.last_check,
                health.response_time_ms,
                health.error_message,
            ],
        )?;
        Ok(())
    }

    /// 获取服务器健康状态
    pub fn get_health(&self, server_id: &str) -> Result<Option<McpHealth>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT server_id, status, last_check, response_time_ms, error_message
             FROM mcp_health
             WHERE server_id = ?1",
        )?;

        let result = stmt.query_row(params![server_id], |row| {
            let status_str: String = row.get(1)?;
            let status = match status_str.as_str() {
                "Healthy" => HealthStatus::Healthy,
                "Degraded" => HealthStatus::Degraded,
                _ => HealthStatus::Down,
            };

            Ok(McpHealth {
                server_id: row.get(0)?,
                status,
                last_check: row.get(2)?,
                response_time_ms: row.get(3)?,
                error_message: row.get(4).ok(),
            })
        });

        match result {
            Ok(health) => Ok(Some(health)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// 检查服务器健康（ping）
    pub async fn check_server(
        &self,
        server_id: &str,
        command: &str,
        args: &[String],
    ) -> McpHealth {
        let start = SystemTime::now();
        let now = start
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // 尝试发现工具（作为健康检查）
        match tokio::time::timeout(
            Duration::from_secs(5),
            crate::mcp::server::discover_tools(command.to_string(), args.to_vec()),
        )
        .await
        {
            Ok(Ok(_)) => {
                let response_time = start.elapsed().unwrap().as_millis() as i64;
                McpHealth {
                    server_id: server_id.to_string(),
                    status: HealthStatus::Healthy,
                    last_check: now,
                    response_time_ms: response_time,
                    error_message: None,
                }
            }
            Ok(Err(e)) => McpHealth {
                server_id: server_id.to_string(),
                status: HealthStatus::Down,
                last_check: now,
                response_time_ms: 0,
                error_message: Some(e),
            },
            Err(_) => McpHealth {
                server_id: server_id.to_string(),
                status: HealthStatus::Down,
                last_check: now,
                response_time_ms: 5000,
                error_message: Some("超时".to_string()),
            },
        }
    }
}

impl Default for McpHealthManager {
    fn default() -> Self {
        Self::new().expect("无法初始化 MCP 健康检查管理器")
    }
}

/// 全局 MCP 健康检查管理器
static MCP_HEALTH_MANAGER: std::sync::LazyLock<McpHealthManager> =
    std::sync::LazyLock::new(|| McpHealthManager::new().expect("无法初始化 MCP 健康检查管理器"));

pub fn global_mcp_health() -> &'static McpHealthManager {
    &MCP_HEALTH_MANAGER
}

#[cfg(test)]
#[path = "health_tests.rs"]
mod tests;

