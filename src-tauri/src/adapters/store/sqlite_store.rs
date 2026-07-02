use rusqlite::{Connection, params};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::path::Path;
use crate::core::models::{Thread, Turn, ThreadTodo, ThreadMode, ThreadStatus, TurnStatus};
use crate::ports::{ThreadStore, StoreResult, StoreError};

pub struct SqliteThreadStore {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteThreadStore {
    pub fn new(database_url: &str) -> Result<Self, rusqlite::Error> {
        let path = database_url.strip_prefix("sqlite:").unwrap_or(database_url);
        let conn = Connection::open(path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS threads (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                workspace_path TEXT,
                status TEXT NOT NULL,
                mode TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                pinned INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        // 老库迁移：为已存在的 threads 表补 pinned 列。
        // 新库建表已含该列，此处 ALTER 会因「列已存在」报错，忽略即可。
        let _ = conn.execute("ALTER TABLE threads ADD COLUMN pinned INTEGER NOT NULL DEFAULT 0", []);

        conn.execute(
            "CREATE TABLE IF NOT EXISTS turns (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                status TEXT NOT NULL,
                model TEXT NOT NULL,
                data TEXT NOT NULL,
                FOREIGN KEY (thread_id) REFERENCES threads(id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS todos (
                id TEXT PRIMARY KEY,
                thread_id TEXT NOT NULL,
                text TEXT NOT NULL,
                completed INTEGER NOT NULL,
                plan_linked INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (thread_id) REFERENCES threads(id)
            )",
            [],
        )?;

        conn.execute("CREATE INDEX IF NOT EXISTS idx_threads_updated ON threads(updated_at DESC)", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_turns_thread ON turns(thread_id)", [])?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }
}

#[async_trait]
impl ThreadStore for SqliteThreadStore {
    async fn create_thread(&self, thread: &Thread) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        conn.execute(
            "INSERT INTO threads (id, title, workspace_path, status, mode, created_at, updated_at, pinned)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                thread.id,
                thread.title,
                thread.workspace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                format!("{:?}", thread.status).to_lowercase(),
                format!("{:?}", thread.mode).to_lowercase(),
                thread.created_at.to_rfc3339(),
                thread.updated_at.to_rfc3339(),
                thread.pinned as i32,
            ],
        ).map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn get_thread(&self, id: &str) -> StoreResult<Thread> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        let mut stmt = conn.prepare("SELECT id, title, workspace_path, status, mode, created_at, updated_at, pinned FROM threads WHERE id = ?")
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        let thread = stmt.query_row(params![id], |row| {
            Ok(Thread {
                id: row.get(0)?,
                title: row.get(1)?,
                workspace_path: row.get::<_, Option<String>>(2)?.map(|p| p.into()),
                status: match row.get::<_, String>(3)?.as_str() {
                    "active" => ThreadStatus::Active,
                    "archived" => ThreadStatus::Archived,
                    _ => ThreadStatus::Deleted,
                },
                mode: match row.get::<_, String>(4)?.as_str() {
                    "write" => ThreadMode::Write,
                    "review" => ThreadMode::Review,
                    _ => ThreadMode::Chat,
                },
                goal: None,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                pinned: row.get::<_, i32>(7)? != 0,
            })
        }).map_err(|_| StoreError::NotFound(id.to_string()))?;

        Ok(thread)
    }

    async fn update_thread(&self, thread: &Thread) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        conn.execute(
            "UPDATE threads SET title = ?, workspace_path = ?, status = ?, mode = ?, updated_at = ?, pinned = ? WHERE id = ?",
            params![
                thread.title,
                thread.workspace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                format!("{:?}", thread.status).to_lowercase(),
                format!("{:?}", thread.mode).to_lowercase(),
                thread.updated_at.to_rfc3339(),
                thread.pinned as i32,
                thread.id,
            ],
        ).map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn list_threads(&self, limit: usize, offset: usize) -> StoreResult<Vec<Thread>> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT id, title, workspace_path, status, mode, created_at, updated_at, pinned FROM threads WHERE status != 'deleted' ORDER BY pinned DESC, updated_at DESC LIMIT ? OFFSET ?"
        ).map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        let threads = stmt.query_map(params![limit, offset], |row| {
            Ok(Thread {
                id: row.get(0)?,
                title: row.get(1)?,
                workspace_path: row.get::<_, Option<String>>(2)?.map(|p| p.into()),
                status: match row.get::<_, String>(3)?.as_str() {
                    "active" => ThreadStatus::Active,
                    "archived" => ThreadStatus::Archived,
                    _ => ThreadStatus::Deleted,
                },
                mode: match row.get::<_, String>(4)?.as_str() {
                    "write" => ThreadMode::Write,
                    "review" => ThreadMode::Review,
                    _ => ThreadMode::Chat,
                },
                goal: None,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
                pinned: row.get::<_, i32>(7)? != 0,
            })
        }).map_err(|e| StoreError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        Ok(threads)
    }

    async fn delete_thread(&self, id: &str) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        conn.execute("UPDATE threads SET status = 'deleted' WHERE id = ?", params![id])
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn save_turn(&self, turn: &Turn) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        let data = serde_json::to_string(turn).map_err(|e| StoreError::SerializationError(e.to_string()))?;
        conn.execute(
            "INSERT INTO turns (id, thread_id, status, model, data) VALUES (?, ?, ?, ?, ?)",
            params![
                turn.id,
                turn.thread_id,
                format!("{:?}", turn.status).to_lowercase(),
                turn.model,
                data,
            ],
        ).map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn get_turn(&self, id: &str) -> StoreResult<Turn> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        let data: String = conn.query_row("SELECT data FROM turns WHERE id = ?", params![id], |row| row.get(0))
            .map_err(|_| StoreError::NotFound(id.to_string()))?;
        serde_json::from_str(&data).map_err(|e| StoreError::SerializationError(e.to_string()))
    }

    async fn list_turns(&self, thread_id: &str, limit: usize) -> StoreResult<Vec<Turn>> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        let mut stmt = conn.prepare("SELECT data FROM turns WHERE thread_id = ? ORDER BY id DESC LIMIT ?")
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        let turns = stmt.query_map(params![thread_id, limit], |row| {
            let data: String = row.get(0)?;
            Ok(data)
        }).map_err(|e| StoreError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StoreError::DatabaseError(e.to_string()))?
        .into_iter()
        .filter_map(|data| serde_json::from_str(&data).ok())
        .collect();

        Ok(turns)
    }

    async fn delete_turns_for_thread(&self, thread_id: &str) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        conn.execute("DELETE FROM turns WHERE thread_id = ?", params![thread_id])
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn save_todo(&self, todo: &ThreadTodo) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        conn.execute(
            "INSERT INTO todos (id, thread_id, text, completed, plan_linked, created_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                todo.id,
                todo.thread_id,
                todo.text,
                todo.completed as i32,
                todo.plan_linked as i32,
                todo.created_at.to_rfc3339(),
            ],
        ).map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn list_todos(&self, thread_id: &str) -> StoreResult<Vec<ThreadTodo>> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        let mut stmt = conn.prepare("SELECT * FROM todos WHERE thread_id = ? ORDER BY created_at")
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        let todos = stmt.query_map(params![thread_id], |row| {
            Ok(ThreadTodo {
                id: row.get(0)?,
                thread_id: row.get(1)?,
                text: row.get(2)?,
                completed: row.get::<_, i32>(3)? != 0,
                plan_linked: row.get::<_, i32>(4)? != 0,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(|_| chrono::Utc::now()),
            })
        }).map_err(|e| StoreError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| StoreError::DatabaseError(e.to_string()))?;

        Ok(todos)
    }

    async fn update_todo(&self, todo: &ThreadTodo) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        conn.execute(
            "UPDATE todos SET text = ?, completed = ?, plan_linked = ? WHERE id = ?",
            params![todo.text, todo.completed as i32, todo.plan_linked as i32, todo.id],
        ).map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn delete_todo(&self, id: &str) -> StoreResult<()> {
        let conn = self.conn.lock().map_err(|e| StoreError::DatabaseError(format!("锁 poisoned: {}", e)))?;
        conn.execute("DELETE FROM todos WHERE id = ?", params![id])
            .map_err(|e| StoreError::DatabaseError(e.to_string()))?;
        Ok(())
    }
}
