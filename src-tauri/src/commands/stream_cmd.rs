use crate::commands::chat::ChatRequest;
use crate::providers::types::StreamChunk;
use crate::providers::openai_compat::OpenAICompatProvider;
use crate::providers::anthropic::AnthropicProvider;
use futures_util::FutureExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

lazy_static::lazy_static! {
    static ref STREAM_SESSIONS: Arc<Mutex<HashMap<String, mpsc::UnboundedReceiver<StreamChunk>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamResponse {
    pub chunks: Vec<StreamChunk>,
    pub done: bool,
}

/// 启动流式聊天会话
#[tauri::command]
pub async fn start_stream_chat(request: ChatRequest) -> Result<String, String> {
    let session_id = format!("stream_{}", chrono::Utc::now().timestamp_millis());
    let (tx, rx) = mpsc::unbounded_channel();

    // 保存接收端
    STREAM_SESSIONS.lock().unwrap().insert(session_id.clone(), rx);

    // 在后台运行聊天（捕获 panic 以便诊断）
    let tx_panic = tx.clone();
    tokio::spawn(async move {
        match std::panic::AssertUnwindSafe(run_chat_with_adapter(request, tx))
            .catch_unwind()
            .await
        {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => {
                eprintln!("[start_stream_chat] Error: {}", e);
            }
            Err(panic_err) => {
                let panic_msg = if let Some(s) = panic_err.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_err.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "unknown panic".to_string()
                };
                eprintln!("[start_stream_chat] PANIC: {}", panic_msg);
                let _ = tx_panic.send(StreamChunk::Error {
                    message: format!("内部错误（panic）: {}", panic_msg),
                });
                let _ = tx_panic.send(StreamChunk::Done {
                    finish_reason: Some("error".into()),
                });
            }
        }
    });

    Ok(session_id)
}

/// 轮询获取流式消息
#[tauri::command]
pub async fn poll_stream_chunks(session_id: String) -> Result<StreamResponse, String> {
    let mut sessions = STREAM_SESSIONS.lock().unwrap();

    if let Some(rx) = sessions.get_mut(&session_id) {
        let mut chunks = Vec::new();
        let mut done = false;

        // 非阻塞地收集所有可用的消息，并检测 channel 断开
        loop {
            match rx.try_recv() {
                Ok(chunk) => {
                    if matches!(chunk, StreamChunk::Done { .. }) {
                        done = true;
                        chunks.push(chunk);
                        break;
                    }
                    chunks.push(chunk);
                }
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    eprintln!("[poll_stream_chunks] Channel disconnected for session {}", session_id);
                    if chunks.is_empty() {
                        chunks.push(StreamChunk::Error {
                            message: "连接中断：后台任务异常终止".into(),
                        });
                    }
                    chunks.push(StreamChunk::Done {
                        finish_reason: Some("error".into()),
                    });
                    done = true;
                    break;
                }
            }
        }

        // 如果会话结束，清理
        if done {
            sessions.remove(&session_id);
        }

        Ok(StreamResponse { chunks, done })
    } else {
        Err("Session not found".into())
    }
}

async fn run_chat_with_adapter(
    request: ChatRequest,
    tx: mpsc::UnboundedSender<StreamChunk>,
) -> Result<(), String> {
    eprintln!("[adapter] 开始处理: provider={}, model={}", request.provider_id, request.model);

    let has_creds = request.api_key.is_some() && request.api_base.is_some();

    if !has_creds {
        eprintln!("[adapter] 缺少 API 配置");
        let _ = tx.send(StreamChunk::Error { message: "缺少 API 配置".into() });
        let _ = tx.send(StreamChunk::Done { finish_reason: Some("error".into()) });
        return Err("缺少 API 配置".into());
    }

    let api_key = request.api_key.unwrap();
    let api_base = request.api_base.unwrap();
    let provider_type = request.provider_type.as_deref().unwrap_or("openai-compat");
    let work_dir = request.work_dir.unwrap_or_default();

    eprintln!("[adapter] provider_type={}, api_base={}", provider_type, api_base);

    let adapter = ChannelAdapter { tx: tx.clone() };

    let result = match provider_type {
        "openai-compat" => {
            let provider = OpenAICompatProvider::new(api_base, api_key);
            crate::commands::chat::run_agent_loop(
                &provider, request.messages, &request.model, adapter, &work_dir,
            ).await
        }
        "anthropic" => {
            let provider = AnthropicProvider::new(api_base, api_key);
            crate::commands::chat::run_agent_loop(
                &provider, request.messages, &request.model, adapter, &work_dir,
            ).await
        }
        _ => {
            let msg = format!("不支持的 Provider: {}", provider_type);
            eprintln!("[adapter] {}", msg);
            let _ = tx.send(StreamChunk::Error { message: msg.clone() });
            let _ = tx.send(StreamChunk::Done { finish_reason: Some("error".into()) });
            return Err(msg);
        }
    };

    // 无论成功还是失败，都确保发送 Done（防止 agent 内部漏发导致 channel 断开）
    match result {
        Ok(_) => {
            eprintln!("[adapter] agent 返回 Ok，确保发送 Done");
            let _ = tx.send(StreamChunk::Done { finish_reason: Some("stop".into()) });
            Ok(())
        }
        Err(e) => {
            eprintln!("[adapter] agent 返回 Err: {}", e);
            let _ = tx.send(StreamChunk::Error { message: e.to_string() });
            let _ = tx.send(StreamChunk::Done { finish_reason: Some("error".into()) });
            Err(e.to_string())
        }
    }
}

/// Channel 适配器 - 实现类似 tauri::ipc::Channel 的接口
#[derive(Clone)]
pub struct ChannelAdapter {
    tx: mpsc::UnboundedSender<StreamChunk>,
}

impl ChannelAdapter {
    pub fn send(&self, chunk: StreamChunk) -> Result<(), String> {
        self.tx.send(chunk).map_err(|e| e.to_string())
    }
}
