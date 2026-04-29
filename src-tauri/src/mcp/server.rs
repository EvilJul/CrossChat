use crate::providers::types::ToolDefinition;
use serde_json::Value;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};

/// 发现 MCP 服务器的工具列表 (tools/list)
pub async fn discover_tools(
    command: String,
    args: Vec<String>,
) -> Result<Vec<ToolDefinition>, String> {
    let init_req = serde_json::json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "crosschat", "version": "0.1.0" }
        }
    });
    let tools_req = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}
    });

    let response = do_mcp_handshake_and_request(&command, &args, &init_req, &tools_req).await?;

    let tools = response["result"]["tools"]
        .as_array()
        .ok_or_else(|| format!("MCP tools/list 返回格式无效: {}", response))?;

    let mut definitions = Vec::new();
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("").to_string();
        let description = tool["description"].as_str().unwrap_or("").to_string();
        let parameters = tool.get("inputSchema").cloned().unwrap_or(serde_json::json!({
            "type": "object", "properties": {}
        }));
        if !name.is_empty() {
            definitions.push(ToolDefinition {
                name: format!("mcp_{}", name),
                description: format!("[MCP] {}", description),
                parameters,
            });
        }
    }
    Ok(definitions)
}

/// 调用 MCP 工具 (tools/call)
pub async fn call_tool(
    command: String,
    args: Vec<String>,
    tool_name: &str,
    arguments: &Value,
) -> Result<String, String> {
    let real_name = tool_name.strip_prefix("mcp_").unwrap_or(tool_name);

    // 启动进程并完成 MCP 握手
    let init_req = serde_json::json!({
        "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "crosschat", "version": "0.1.0" }
        }
    });

    let tool_req = serde_json::json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/call",
        "params": { "name": real_name, "arguments": arguments }
    });

    let response = do_mcp_handshake_and_request(&command, &args, &init_req, &tool_req).await?;

    if let Some(err) = response.get("error") {
        return Err(format!("MCP 工具调用错误: {}", err));
    }
    Ok(response["result"]["content"]
        .as_array()
        .map(|items| {
            items.iter()
                .filter_map(|i| i["text"].as_str().or(i["value"].as_str()))
                .collect::<Vec<_>>().join("\n")
        })
        .unwrap_or_else(|| response["result"].to_string()))
}

/// 单次 MCP 会话：启动进程 → 握手 → 发请求 → 读响应 → 关进程
async fn do_mcp_handshake_and_request(
    command: &str, args: &[String],
    init_req: &Value,
    actual_req: &Value,
) -> Result<Value, String> {
    let (mut child, mut stdin, stdout) = spawn_mcp(command, args).await?;
    let mut reader = BufReader::new(stdout);
    let mut line_buf = String::new();

    // 1. 发送 initialize 请求
    write_request(&mut stdin, init_req).await?;
    // 2. 读取 initialize 响应
    let _init = read_response_from_reader(&mut reader, &mut line_buf, 30).await?;

    // 3. 发送 initialized 通知（JSON-RPC 通知：无 id，无需响应）
    let notif = serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"});
    write_request(&mut stdin, &notif).await?;

    // 4. 发送实际请求（tools/list 或 tools/call）
    write_request(&mut stdin, actual_req).await?;

    // 5. 读取响应
    let response = read_response_from_reader(&mut reader, &mut line_buf, 20).await?;

    // 6. 清理
    let _ = tokio::time::timeout(std::time::Duration::from_secs(3), child.wait()).await;
    child.kill().await.ok();

    Ok(response)
}

/// 启动 MCP 子进程
async fn spawn_mcp(
    command: &str, args: &[String],
) -> Result<(Child, ChildStdin, tokio::process::ChildStdout), String> {
    #[cfg(target_os = "windows")]
    let (cmd, cmd_args) = {
        if command == "npx" {
            let mut a = vec!["/c".to_string(), "npx".to_string()];
            a.extend(args.iter().cloned());
            ("cmd".to_string(), a)
        } else if command == "uvx" || command == "uv" {
            let mut a = vec!["/c".to_string(), command.to_string()];
            a.extend(args.iter().cloned());
            ("cmd".to_string(), a)
        } else {
            (command.to_string(), args.to_vec())
        }
    };
    #[cfg(not(target_os = "windows"))]
    let (cmd, cmd_args) = (command.to_string(), args.to_vec());

    let mut cmd_builder = tokio::process::Command::new(&cmd);
    cmd_builder
        .args(&cmd_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("PUPPETEER_HEADLESS", "true")
        .env("HEADLESS", "true")
        .env("BROWSER_HEADLESS", "true");

    // Windows: 不弹出控制台窗口
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd_builder.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let mut child = cmd_builder
        .spawn()
        .map_err(|e| format!("无法启动 MCP 进程 {}: {}", command, e))?;

    let stdin = child.stdin.take().ok_or("无法获取 stdin")?;
    let stdout = child.stdout.take().ok_or("无法获取 stdout")?;
    Ok((child, stdin, stdout))
}

/// 向子进程写入 JSON 请求
async fn write_request(stdin: &mut ChildStdin, req: &Value) -> Result<(), String> {
    let s = serde_json::to_string(req).map_err(|e| e.to_string())?;
    stdin.write_all(s.as_bytes()).await.map_err(|e| format!("写入失败: {}", e))?;
    stdin.write_all(b"\n").await.map_err(|e| format!("写入换行失败: {}", e))?;
    Ok(())
}

/// 从子进程读取多行 JSON 响应
#[allow(dead_code)]
async fn read_response(
    mut reader: BufReader<tokio::process::ChildStdout>,
    timeout_secs: u64,
) -> Result<Value, String> {
    let mut line_buf = String::new();
    read_response_from_reader(&mut reader, &mut line_buf, timeout_secs).await
}

// Unused: reader parameter needed for compatibility with read_response
#[allow(unused_mut)]
async fn read_response_from_reader(
    reader: &mut BufReader<tokio::process::ChildStdout>,
    line_buf: &mut String,
    timeout_secs: u64,
) -> Result<Value, String> {
    line_buf.clear();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        async {
            let mut lines = reader.lines();
            loop {
                match lines.next_line().await {
                    Ok(Some(line)) => {
                        line_buf.push_str(&line);
                        if let Ok(val) = serde_json::from_str::<Value>(line_buf) {
                            return Ok(val);
                        }
                        if line_buf.lines().count() > 50 {
                            return Err(format!("响应行数过多: {}...", &line_buf[..200.min(line_buf.len())]));
                        }
                    }
                    Ok(None) => return Err("MCP 服务器未返回响应".into()),
                    Err(e) => return Err(format!("读取失败: {}", e)),
                }
            }
        },
    ).await;

    match result {
        Ok(Ok(val)) => Ok(val),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!("MCP 请求超时 ({}秒)", timeout_secs)),
    }
}
