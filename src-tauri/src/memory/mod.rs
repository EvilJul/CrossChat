pub mod vector;

use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use vector::VectorSearch;

/// 记忆条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: Option<i64>,
    pub task: String,
    pub solution: String,
    pub tools_used: String,
    pub success: bool,
    pub timestamp: i64,
    #[serde(default)]
    pub failure_reason: Option<String>,
    #[serde(default)]
    pub fix_applied: Option<String>,
}

/// 记忆管理器
pub struct MemoryManager {
    db_path: PathBuf,
}

impl MemoryManager {
    pub fn new() -> Result<Self> {
        let home = std::env::var("APPDATA")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".into());
        let dir = PathBuf::from(&home).join(".crosschat");
        std::fs::create_dir_all(&dir).ok();
        let db_path = dir.join("memory.db");

        let mgr = Self { db_path };
        mgr.init_db()?;
        Ok(mgr)
    }

    fn init_db(&self) -> Result<()> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                task TEXT NOT NULL,
                solution TEXT NOT NULL,
                tools_used TEXT NOT NULL,
                success INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                failure_reason TEXT,
                fix_applied TEXT
            )",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_task ON memories(task)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_timestamp ON memories(timestamp DESC)",
            [],
        )?;
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_success ON memories(success)",
            [],
        )?;
        Ok(())
    }

    /// 保存记忆
    pub fn save(&self, memory: &Memory) -> Result<i64> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute(
            "INSERT INTO memories (task, solution, tools_used, success, timestamp, failure_reason, fix_applied)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                memory.task,
                memory.solution,
                memory.tools_used,
                memory.success as i32,
                memory.timestamp,
                memory.failure_reason,
                memory.fix_applied,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 搜索相似任务（向量检索）
    pub fn search_vector(&self, query: &str, limit: usize) -> Result<Vec<Memory>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, task, solution, tools_used, success, timestamp, failure_reason, fix_applied
             FROM memories
             WHERE success = 1
             ORDER BY timestamp DESC
             LIMIT 100", // 先取最近 100 条
        )?;

        let memories: Vec<Memory> = stmt.query_map([], |row| {
            Ok(Memory {
                id: Some(row.get(0)?),
                task: row.get(1)?,
                solution: row.get(2)?,
                tools_used: row.get(3)?,
                success: row.get::<_, i32>(4)? != 0,
                timestamp: row.get(5)?,
                failure_reason: row.get(6).ok(),
                fix_applied: row.get(7).ok(),
            })
        })?.collect::<Result<Vec<_>>>()?;

        // 向量检索
        let candidates: Vec<(String, String)> = memories
            .iter()
            .map(|m| (m.task.clone(), m.solution.clone()))
            .collect();

        let results = VectorSearch::search_similar(query, &candidates, limit);

        Ok(results.into_iter().map(|(idx, _score)| memories[idx].clone()).collect())
    }

    /// 搜索相似任务（文本匹配，保留兼容）
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Memory>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, task, solution, tools_used, success, timestamp, failure_reason, fix_applied
             FROM memories
             WHERE success = 1 AND (task LIKE ?1 OR solution LIKE ?1)
             ORDER BY timestamp DESC
             LIMIT ?2",
        )?;

        let pattern = format!("%{}%", query);
        let memories = stmt.query_map(params![pattern, limit], |row| {
            Ok(Memory {
                id: Some(row.get(0)?),
                task: row.get(1)?,
                solution: row.get(2)?,
                tools_used: row.get(3)?,
                success: row.get::<_, i32>(4)? != 0,
                timestamp: row.get(5)?,
                failure_reason: row.get(6).ok(),
                fix_applied: row.get(7).ok(),
            })
        })?;

        memories.collect()
    }

    /// 获取失败记忆（用于学习）
    pub fn get_failures(&self, limit: usize) -> Result<Vec<Memory>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, task, solution, tools_used, success, timestamp, failure_reason, fix_applied
             FROM memories
             WHERE success = 0
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let memories = stmt.query_map(params![limit], |row| {
            Ok(Memory {
                id: Some(row.get(0)?),
                task: row.get(1)?,
                solution: row.get(2)?,
                tools_used: row.get(3)?,
                success: row.get::<_, i32>(4)? != 0,
                timestamp: row.get(5)?,
                failure_reason: row.get(6).ok(),
                fix_applied: row.get(7).ok(),
            })
        })?;

        memories.collect()
    }

    /// 获取最近的成功记忆
    pub fn get_recent(&self, limit: usize) -> Result<Vec<Memory>> {
        let conn = Connection::open(&self.db_path)?;
        let mut stmt = conn.prepare(
            "SELECT id, task, solution, tools_used, success, timestamp, failure_reason, fix_applied
             FROM memories
             WHERE success = 1
             ORDER BY timestamp DESC
             LIMIT ?1",
        )?;

        let memories = stmt.query_map(params![limit], |row| {
            Ok(Memory {
                id: Some(row.get(0)?),
                task: row.get(1)?,
                solution: row.get(2)?,
                tools_used: row.get(3)?,
                success: row.get::<_, i32>(4)? != 0,
                timestamp: row.get(5)?,
                failure_reason: row.get(6).ok(),
                fix_applied: row.get(7).ok(),
            })
        })?;

        memories.collect()
    }

    /// 清理旧记忆（保留最近 N 条）
    pub fn cleanup(&self, keep_count: usize) -> Result<usize> {
        let conn = Connection::open(&self.db_path)?;
        let deleted = conn.execute(
            "DELETE FROM memories WHERE id NOT IN (
                SELECT id FROM memories ORDER BY timestamp DESC LIMIT ?1
            )",
            params![keep_count],
        )?;
        Ok(deleted)
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new().expect("无法初始化记忆管理器")
    }
}

/// 全局记忆管理器
static MEMORY_MANAGER: std::sync::LazyLock<MemoryManager> =
    std::sync::LazyLock::new(|| MemoryManager::new().expect("无法初始化记忆管理器"));

pub fn global_memory() -> &'static MemoryManager {
    &MEMORY_MANAGER
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

