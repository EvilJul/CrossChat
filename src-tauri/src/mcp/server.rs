use crate::providers::types::ToolDefinition;
use serde_json::Value;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command as TokioCommand;

/// 发现 MCP 服务器的工具列表 (tools/list)
pub async fn discover_tools(
    command: String,
    args: Vec<String>,
) -> Result<Vec<ToolDefinition>, String> {
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = send_mcp_request(&command, &args, &request).await?;
    let tools = response["result"]["tools"]
        .as_array()
        .ok_or_else(|| format!("MCP tools/list 返回格式无效: {}", response))?;

    let mut definitions = Vec::new();
    for tool in tools {
        let name = tool["name"].as_str().unwrap_or("").to_string();
        let description = tool["description"].as_str().unwrap_or("").to_string();
        let parameters = tool.get("inputSchema").cloned().unwrap_or(serde_json::json!({
            "type": "object",
            "properties": {}
        }));

        if !name.is_empty() {
            definitions.push(ToolDefinition {
                name: format!("mcp_{}", name), // MCP 工具加前缀避免冲突
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
    // 去掉 mcp_ 前缀
    let real_name = tool_name.strip_prefix("mcp_").unwrap_or(tool_name);

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": real_name,
            "arguments": arguments
        }
    });

    let response = send_mcp_request(&command, &args, &request).await?;

    if let Some(err) = response.get("error") {
        return Err(format!("MCP 工具调用错误: {}", err));
    }

    Ok(response["result"]["content"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|i| i["text"].as_str().or(i["value"].as_str()))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_else(|| response["result"].to_string()))
}

/// 发送 JSON-RPC 请求到 MCP 服务器并读取响应
async fn send_mcp_request(
    command: &str,
    args: &[String],
    request: &Value,
) -> Result<Value, String> {
    #[cfg(target_os = "windows")]
    let (cmd, cmd_args) = {
        if command == "npx" {
            // Windows 下 npx 可能需要用 npx.cmd 或 cmd /c
            let mut a = vec!["/c".to_string(), "npx".to_string()];
            a.extend(args.iter().cloned());
            ("cmd".to_string(), a)
        } else {
            (command.to_string(), args.to_vec())
        }
    };

    #[cfg(not(target_os = "windows"))]
    let (cmd, cmd_args) = (command.to_string(), args.to_vec());

    let mut child = TokioCommand::new(&cmd)
        .args(&cmd_args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("无法启动 MCP 服务进程 {}: {}", command, e))?;

    let mut stdin = child.stdin.take().ok_or("无法获取 stdin")?;
    let stdout = child.stdout.take().ok_or("无法获取 stdout")?;

    // 发送请求
    let request_str = serde_json::to_string(request).map_err(|e| e.to_string())?;
    stdin
        .write_all(request_str.as_bytes())
        .await
        .map_err(|e| format!("写入请求失败: {}", e))?;
    stdin
        .write_all(b"\n")
        .await
        .map_err(|e| format!("写入换行失败: {}", e))?;
    drop(stdin);

    // 读取响应（带超时）
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(15),
        lines.next_line(),
    )
    .await;

    match result {
        Ok(Ok(Some(line))) => {
            // 等待进程退出
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), child.wait()).await;
            serde_json::from_str(&line).map_err(|e| format!("解析 MCP 响应失败: {} - 内容: {}", e, line))
        }
        Ok(Ok(None)) => Err("MCP 服务器未返回响应".into()),
        Ok(Err(e)) => Err(format!("读取 MCP 响应失败: {}", e)),
        Err(_) => {
            child.kill().await.ok();
            Err("MCP 请求超时 (15秒)".into())
        }
    }
}
