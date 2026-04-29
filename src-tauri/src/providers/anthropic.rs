use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::Value;
use tauri::ipc::Channel;

use super::{LlmProvider, ProviderError, ProviderType};
use super::types::{
    ChatSyncResult, ContentBlock, ImageSource, MessageRole, StreamChunk, ToolCall,
    ToolDefinition, UnifiedMessage,
};
use crate::streaming::sse_parser::SseParser;

/// Anthropic Provider — 实现 Anthropic Messages API
/// 文档: https://docs.anthropic.com/en/api/messages
pub struct AnthropicProvider {
    pub api_base: String,
    pub api_key: String,
    pub client: reqwest::Client,
}

impl AnthropicProvider {
    pub fn new(api_base: String, api_key: String) -> Self {
        Self {
            api_base: api_base.trim_end_matches('/').to_string(),
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    /// 将统一消息转为 Anthropic 格式
    /// 返回 (system_prompt, messages)
    fn convert_messages(
        &self,
        messages: Vec<UnifiedMessage>,
    ) -> (Option<Vec<Value>>, Vec<Value>) {
        let mut system_blocks: Vec<Value> = Vec::new();
        let mut anthropic_messages: Vec<Value> = Vec::new();

        for msg in messages {
            let text: String = msg
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    ContentBlock::Image { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("");

            match msg.role {
                MessageRole::User => {
                    // 检测系统指令类消息（以 [系统 或 [上下文 开头）
                    if text.starts_with("[系统指令") || text.starts_with("[上下文摘要") {
                        let clean_text = text
                            .replacen("[系统指令 — 遵循以下规则约束你的行为]\n\n", "", 1)
                            .replacen("[上下文摘要 — 之前对话的要点]\n", "", 1)
                            .trim()
                            .to_string();
                        system_blocks.push(serde_json::json!({
                            "type": "text",
                            "text": clean_text,
                        }));
                    } else {
                        // 构建 Anthropic 格式的 content 数组
                        let mut content_parts: Vec<Value> = Vec::new();
                        for block in &msg.content {
                            match block {
                                ContentBlock::Text { text: t } => {
                                    if !t.is_empty() {
                                        content_parts.push(serde_json::json!({
                                            "type": "text", "text": t
                                        }));
                                    }
                                }
                                ContentBlock::Image { source } => {
                                    let (media_type, data) = match source {
                                        ImageSource::DataUrl { url } => {
                                            // 解析 data:image/png;base64,xxx
                                            let mime = url.split(";base64,").next()
                                                .and_then(|s| s.strip_prefix("data:"))
                                                .unwrap_or("image/png");
                                            let b64 = url.split(";base64,").nth(1).unwrap_or("");
                                            (mime.to_string(), b64.to_string())
                                        }
                                        ImageSource::FilePath { path, mime_type } => {
                                            let mime = mime_type.clone().unwrap_or_else(||
                                                mime_guess::from_path(path).first_or_octet_stream().to_string()
                                            );
                                            let b64 = crate::providers::openai_compat::read_file_base64(path).unwrap_or_default();
                                            (mime, b64)
                                        }
                                    };
                                    content_parts.push(serde_json::json!({
                                        "type": "image",
                                        "source": {
                                            "type": "base64",
                                            "media_type": media_type,
                                            "data": data,
                                        }
                                    }));
                                }
                            }
                        }
                        anthropic_messages.push(serde_json::json!({
                            "role": "user",
                            "content": content_parts,
                        }));
                    }
                }
                MessageRole::Assistant => {
                    let mut content_blocks: Vec<Value> = Vec::new();

                    // 文本内容
                    if !text.is_empty() {
                        content_blocks.push(serde_json::json!({
                            "type": "text",
                            "text": text,
                        }));
                    }

                    // 工具调用 → Anthropic tool_use 格式
                    if let Some(tool_calls) = &msg.tool_calls {
                        for tc in tool_calls {
                            content_blocks.push(serde_json::json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.name,
                                "input": tc.arguments,
                            }));
                        }
                    }

                    if !content_blocks.is_empty() {
                        anthropic_messages.push(serde_json::json!({
                            "role": "assistant",
                            "content": content_blocks,
                        }));
                    }
                }
                MessageRole::Tool => {
                    // Anthropic: 工具结果必须是 user 角色
                    let tool_use_id = msg.tool_call_id.clone().unwrap_or_default();
                    anthropic_messages.push(serde_json::json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": text,
                        }],
                    }));
                }
            }
        }

        let system = if system_blocks.is_empty() {
            None
        } else {
            Some(system_blocks)
        };

        (system, anthropic_messages)
    }

    /// 将统一工具定义转为 Anthropic 格式
    fn convert_tools(&self, tools: &[ToolDefinition]) -> Option<Vec<Value>> {
        if tools.is_empty() {
            return None;
        }
        Some(
            tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters,
                    })
                })
                .collect(),
        )
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
    }

    async fn stream_chat(
        &self,
        messages: Vec<UnifiedMessage>,
        tools: Vec<ToolDefinition>,
        model: &str,
        channel: Channel<StreamChunk>,
    ) -> Result<(), ProviderError> {
        let (system, anthropic_messages) = self.convert_messages(messages);
        let anthropic_tools = self.convert_tools(&tools);

        let mut request_body = serde_json::json!({
            "model": model,
            "max_tokens": 8192,
            "messages": anthropic_messages,
            "stream": true,
        });

        if let Some(sys) = system {
            request_body["system"] = serde_json::json!(sys);
        }
        if let Some(t) = anthropic_tools {
            request_body["tools"] = serde_json::json!(t);
        }

        let url = format!("{}/messages", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        // 解析 Anthropic SSE 流
        let mut stream = response.bytes_stream();
        let mut sse_parser = SseParser::new();
        let mut tool_states: std::collections::HashMap<usize, (String, String)> =
            std::collections::HashMap::new(); // index -> (id, name)

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);
            let events = sse_parser.feed(&text);

            for lines in events {
                let mut event_type = String::new();
                let mut data_str = String::new();

                for line in &lines {
                    if let Some(evt) = line.strip_prefix("event: ") {
                        event_type = evt.trim().to_string();
                    }
                    if let Some(data) = line.strip_prefix("data: ") {
                        data_str = data.to_string();
                    }
                }

                if data_str.is_empty() {
                    continue;
                }

                let parsed: Value = match serde_json::from_str(&data_str) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let evt_type = parsed["type"]
                    .as_str()
                    .unwrap_or(&event_type)
                    .to_string();

                match evt_type.as_str() {
                    "message_start" => {
                        // 新消息开始，可以发送 thinking 事件
                    }
                    "content_block_start" => {
                        let index = parsed["index"].as_u64().unwrap_or(0) as usize;
                        let content_block = &parsed["content_block"];
                        if let Some(block_type) = content_block["type"].as_str() {
                            if block_type == "tool_use" {
                                let id = content_block["id"].as_str().unwrap_or("").to_string();
                                let name =
                                    content_block["name"].as_str().unwrap_or("").to_string();
                                tool_states.insert(index, (id.clone(), name.clone()));
                                let _ = channel.send(StreamChunk::ToolCallStart {
                                    id,
                                    name,
                                });
                            }
                        }
                    }
                    "content_block_delta" => {
                        let index = parsed["index"].as_u64().unwrap_or(0) as usize;
                        let delta = &parsed["delta"];
                        let delta_type = delta["type"].as_str().unwrap_or("");

                        match delta_type {
                            "text_delta" => {
                                if let Some(t) = delta["text"].as_str() {
                                    if !t.is_empty() {
                                        let _ = channel.send(StreamChunk::TextDelta {
                                            delta: t.to_string(),
                                        });
                                    }
                                }
                            }
                            "input_json_delta" => {
                                if let Some(partial) = delta["partial_json"].as_str() {
                                    if !partial.is_empty() {
                                        if let Some((id, _)) = tool_states.get(&index) {
                                            let _ = channel.send(StreamChunk::ToolCallDelta {
                                                id: id.clone(),
                                                arguments_delta: partial.to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                            "thinking_delta" => {
                                // 思考链增量 (extended thinking)
                                if let Some(t) = delta["thinking"].as_str() {
                                    if !t.is_empty() {
                                        let _ = channel.send(StreamChunk::ThinkingDelta {
                                            delta: t.to_string(),
                                        });
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    "content_block_stop" => {
                        let index = parsed["index"].as_u64().unwrap_or(0) as usize;
                        if let Some((id, _)) = tool_states.remove(&index) {
                            let _ =
                                channel.send(StreamChunk::ToolCallEnd { id });
                        }
                    }
                    "message_delta" => {
                        let stop_reason =
                            parsed["delta"]["stop_reason"].as_str().unwrap_or("");
                        if stop_reason == "end_turn" || stop_reason == "stop_sequence" {
                            // 正常结束
                        }
                    }
                    "message_stop" => {
                        let _ = channel.send(StreamChunk::Done {
                            finish_reason: Some("stop".to_string()),
                        });
                        return Ok(());
                    }
                    "error" => {
                        let msg = parsed["error"]["message"]
                            .as_str()
                            .unwrap_or("未知 Anthropic API 错误");
                        let _ = channel.send(StreamChunk::Error {
                            message: msg.to_string(),
                        });
                        return Err(ProviderError::ApiError(msg.to_string()));
                    }
                    _ => {}
                }
            }
        }

        let _ = channel.send(StreamChunk::Done {
            finish_reason: Some("stop".to_string()),
        });
        Ok(())
    }

    async fn chat_sync(
        &self,
        messages: Vec<UnifiedMessage>,
        model: &str,
    ) -> Result<String, ProviderError> {
        let (system, anthropic_messages) = self.convert_messages(messages);

        let mut request_body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "messages": anthropic_messages,
            "stream": false,
        });

        if let Some(sys) = system {
            request_body["system"] = serde_json::json!(sys);
        }

        let url = format!("{}/messages", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let result: Value = response.json().await?;

        // Anthropic 响应: { content: [{ type: "text", text: "..." }] }
        let empty_content = vec![];
        let text = result["content"]
            .as_array()
            .unwrap_or(&empty_content)
            .iter()
            .filter_map(|block| {
                if block["type"].as_str() == Some("text") {
                    block["text"].as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }

    async fn chat_sync_with_tools(
        &self,
        messages: Vec<UnifiedMessage>,
        tools: &[ToolDefinition],
        model: &str,
    ) -> Result<ChatSyncResult, ProviderError> {
        let (system, anthropic_messages) = self.convert_messages(messages);
        let anthropic_tools = self.convert_tools(tools);

        let mut request_body = serde_json::json!({
            "model": model,
            "max_tokens": 8192,
            "messages": anthropic_messages,
            "stream": false,
        });

        if let Some(sys) = system {
            request_body["system"] = serde_json::json!(sys);
        }
        if let Some(t) = anthropic_tools {
            request_body["tools"] = serde_json::json!(t);
        }

        let url = format!("{}/messages", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        let result: Value = response.json().await?;
        let empty_vec = vec![];
        let content_blocks = result["content"].as_array().unwrap_or(&empty_vec);

        let mut text_parts: Vec<String> = Vec::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();

        for block in content_blocks {
            match block["type"].as_str() {
                Some("text") => {
                    if let Some(t) = block["text"].as_str() {
                        text_parts.push(t.to_string());
                    }
                }
                Some("tool_use") => {
                    let id = block["id"].as_str().unwrap_or("").to_string();
                    let name = block["name"].as_str().unwrap_or("").to_string();
                    let input = block["input"].clone();
                    tool_calls.push(ToolCall {
                        id,
                        name,
                        arguments: input,
                    });
                }
                _ => {}
            }
        }

        if !tool_calls.is_empty() {
            Ok(ChatSyncResult::ToolCalls {
                calls: tool_calls,
                reasoning: None,
            })
        } else {
            Ok(ChatSyncResult::Content(text_parts.join("")))
        }
    }
}
