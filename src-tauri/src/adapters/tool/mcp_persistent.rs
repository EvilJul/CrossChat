use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::{oneshot, Mutex};
use std::process::Stdio;

pub struct PersistentMcpClient {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    request_id: Arc<AtomicU64>,
}

impl PersistentMcpClient {
    pub async fn connect(command: &str, args: &[String]) -> Result<Self, String> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to spawn MCP process: {}", e))?;

        let stdin = child.stdin.take().ok_or("Failed to get stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to get stdout")?;

        let stdin = Arc::new(Mutex::new(stdin));
        let pending = Arc::new(Mutex::new(HashMap::new()));
        let request_id = Arc::new(AtomicU64::new(1));

        let mut client = Self {
            child,
            stdin: stdin.clone(),
            pending: pending.clone(),
            request_id: request_id.clone(),
        };

        client.start_reader(stdout, pending.clone());
        client.initialize().await?;

        Ok(client)
    }

    fn start_reader(&self, stdout: ChildStdout, pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>) {
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                if let Ok(msg) = serde_json::from_str::<Value>(&line) {
                    if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
                        if let Some(sender) = pending.lock().await.remove(&id) {
                            let _ = sender.send(msg);
                        }
                    }
                }
            }
        });
    }

    async fn send_request(&self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);

        let mut stdin = self.stdin.lock().await;
        let msg = serde_json::to_string(&request).map_err(|e| e.to_string())?;
        stdin.write_all(msg.as_bytes()).await.map_err(|e| e.to_string())?;
        stdin.write_all(b"\n").await.map_err(|e| e.to_string())?;
        stdin.flush().await.map_err(|e| e.to_string())?;
        drop(stdin);

        let response = rx.await.map_err(|_| "Request cancelled".to_string())?;

        if let Some(error) = response.get("error") {
            return Err(format!("MCP error: {}", error));
        }

        response.get("result")
            .cloned()
            .ok_or_else(|| "No result in response".to_string())
    }

    async fn send_notification(&self, method: &str, params: Value) -> Result<(), String> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let mut stdin = self.stdin.lock().await;
        let msg = serde_json::to_string(&notification).map_err(|e| e.to_string())?;
        stdin.write_all(msg.as_bytes()).await.map_err(|e| e.to_string())?;
        stdin.write_all(b"\n").await.map_err(|e| e.to_string())?;
        stdin.flush().await.map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn initialize(&mut self) -> Result<(), String> {
        self.send_request("initialize", json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "crosschat",
                "version": "0.1.0"
            }
        })).await?;

        self.send_notification("notifications/initialized", json!({})).await?;

        Ok(())
    }

    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value, String> {
        self.send_request("tools/call", json!({
            "name": name,
            "arguments": arguments
        })).await
    }
}

impl Drop for PersistentMcpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}
