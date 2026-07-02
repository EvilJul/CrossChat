use async_trait::async_trait;
use crate::core::models::{Thread, Turn, ThreadTodo};

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Conflict: {0}")]
    Conflict(String),
}

#[async_trait]
pub trait ThreadStore: Send + Sync {
    async fn create_thread(&self, thread: &Thread) -> StoreResult<()>;
    async fn get_thread(&self, id: &str) -> StoreResult<Thread>;
    async fn update_thread(&self, thread: &Thread) -> StoreResult<()>;
    async fn list_threads(&self, limit: usize, offset: usize) -> StoreResult<Vec<Thread>>;
    async fn delete_thread(&self, id: &str) -> StoreResult<()>;

    async fn save_turn(&self, turn: &Turn) -> StoreResult<()>;
    async fn get_turn(&self, id: &str) -> StoreResult<Turn>;
    async fn list_turns(&self, thread_id: &str, limit: usize) -> StoreResult<Vec<Turn>>;
    async fn delete_turns_for_thread(&self, thread_id: &str) -> StoreResult<()>;

    async fn save_todo(&self, todo: &ThreadTodo) -> StoreResult<()>;
    async fn list_todos(&self, thread_id: &str) -> StoreResult<Vec<ThreadTodo>>;
    async fn update_todo(&self, todo: &ThreadTodo) -> StoreResult<()>;
    async fn delete_todo(&self, id: &str) -> StoreResult<()>;
}
