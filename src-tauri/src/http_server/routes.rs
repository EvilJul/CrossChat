use axum::{
    Router,
    routing::{get, post},
    extract::{Path, State},
    response::{Response, sse::{Event, KeepAlive}},
    Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::ports::{ThreadStore, EventBus, StreamEvent};
use crate::application::AgentLoop;
use crate::core::models::{Thread, ThreadMode, Turn};

#[derive(Clone)]
pub struct AppState {
    pub agent_loop: Arc<AgentLoop>,
    pub thread_store: Arc<dyn ThreadStore>,
    pub event_bus: Arc<dyn EventBus>,
}

pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/threads", get(list_threads).post(create_thread))
        .route("/v1/threads/:id", get(get_thread))
        .route("/v1/threads/:id/turns", post(create_turn))
        .route("/v1/threads/:id/events", get(subscribe_events))
        .with_state(state)
}

async fn health() -> &'static str {
    "OK"
}

#[derive(Deserialize)]
struct CreateThreadRequest {
    title: String,
    workspace_path: Option<String>,
    mode: Option<String>,
}

#[derive(Serialize)]
struct ThreadResponse {
    id: String,
    title: String,
    workspace_path: Option<String>,
    status: String,
    mode: String,
    created_at: String,
    updated_at: String,
}

async fn create_thread(
    State(state): State<AppState>,
    Json(req): Json<CreateThreadRequest>,
) -> Result<Json<ThreadResponse>, StatusCode> {
    let mode = match req.mode.as_deref() {
        Some("write") => ThreadMode::Write,
        Some("review") => ThreadMode::Review,
        _ => ThreadMode::Chat,
    };

    let thread = Thread::new(
        req.title,
        req.workspace_path.map(|p| p.into()),
        mode,
    );

    state.thread_store.create_thread(&thread).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ThreadResponse {
        id: thread.id.clone(),
        title: thread.title.clone(),
        workspace_path: thread.workspace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        status: format!("{:?}", thread.status).to_lowercase(),
        mode: format!("{:?}", thread.mode).to_lowercase(),
        created_at: thread.created_at.to_rfc3339(),
        updated_at: thread.updated_at.to_rfc3339(),
    }))
}

async fn list_threads(
    State(state): State<AppState>,
) -> Result<Json<Vec<ThreadResponse>>, StatusCode> {
    let threads = state.thread_store.list_threads(50, 0).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses = threads.into_iter().map(|t| ThreadResponse {
        id: t.id.clone(),
        title: t.title.clone(),
        workspace_path: t.workspace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        status: format!("{:?}", t.status).to_lowercase(),
        mode: format!("{:?}", t.mode).to_lowercase(),
        created_at: t.created_at.to_rfc3339(),
        updated_at: t.updated_at.to_rfc3339(),
    }).collect();

    Ok(Json(responses))
}

async fn get_thread(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ThreadResponse>, StatusCode> {
    let thread = state.thread_store.get_thread(&id).await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(ThreadResponse {
        id: thread.id.clone(),
        title: thread.title.clone(),
        workspace_path: thread.workspace_path.as_ref().map(|p| p.to_string_lossy().to_string()),
        status: format!("{:?}", thread.status).to_lowercase(),
        mode: format!("{:?}", thread.mode).to_lowercase(),
        created_at: thread.created_at.to_rfc3339(),
        updated_at: thread.updated_at.to_rfc3339(),
    }))
}

#[derive(Deserialize)]
struct CreateTurnRequest {
    message: String,
    model: String,
}

#[derive(Serialize)]
struct TurnResponse {
    id: String,
    thread_id: String,
    status: String,
}

async fn create_turn(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
    Json(req): Json<CreateTurnRequest>,
) -> Result<Json<TurnResponse>, StatusCode> {
    let agent_loop = state.agent_loop.clone();
    let thread_id_clone = thread_id.clone();
    let message = req.message.clone();
    let model = req.model.clone();

    tokio::spawn(async move {
        let _ = agent_loop.run_turn(thread_id_clone, message, model).await;
    });

    Ok(Json(TurnResponse {
        id: uuid::Uuid::new_v4().to_string(),
        thread_id,
        status: "running".to_string(),
    }))
}

async fn subscribe_events(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> axum::response::Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.event_bus.subscribe(&thread_id);

    let stream = async_stream::stream! {
        while let Ok(event) = rx.recv().await {
            let event_json = serde_json::to_string(&event).unwrap_or_default();
            yield Ok(Event::default().data(event_json));
        }
    };

    axum::response::Sse::new(stream).keep_alive(KeepAlive::default())
}
