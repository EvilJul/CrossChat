use async_trait::async_trait;
use futures_util::StreamExt;
use serde_json::Value;
use tauri::ipc::Channel;

use super::{LlmProvider, ProviderError, ProviderType};
use super::types::{ChatSyncResult, ContentBlock, ImageSource, MessageRole, StreamChunk, ToolCall, ToolDefinition, UnifiedMessage};
use crate::streaming::sse_parser::SseParser;

/// OpenAI 兼容 Provider
/// 覆盖: OpenAI, DeepSeek, 通义千问(DashScope), Groq, Ollama, LM Studio 等
pub struct OpenAICompatProvider {
    pub api_base: String,
    pub api_key: String,
    pub client: reqwest::Client,
}

impl OpenAICompatProvider {
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

    /// 将统一消息转为 OpenAI 格式
    fn convert_messages(&self, messages: Vec<UnifiedMessage>) -> Vec<Value> {
        messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::Tool => "tool",
                };

                // 检查是否有图片内容
                let has_image = msg.content.iter().any(|block| matches!(block, ContentBlock::Image { .. }));

                // 构建 content 字段
                let content_value = if has_image {
                    // OpenAI Vision 格式: [{type: "text", text: "..."}, {type: "image_url", ...}]
                    let parts: Vec<Value> = msg.content.iter().map(|block| match block {
                        ContentBlock::Text { text } => {
                            serde_json::json!({"type": "text", "text": text})
                        }
                        ContentBlock::Image { source } => {
                            let url = match source {
                                ImageSource::DataUrl { url } => url.clone(),
                                ImageSource::FilePath { path, .. } => {
                                    // 尝试读取文件并转 base64
                                    read_file_as_base64(path).unwrap_or_else(|_| path.clone())
                                }
                            };
                            serde_json::json!({
                                "type": "image_url",
                                "image_url": { "url": url, "detail": "auto" }
                            })
                        }
                    }).collect();
                    serde_json::json!(parts)
                } else {
                    // 纯文本
                    let text: String = msg
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("");
                    serde_json::Value::String(text)
                };

                let mut obj = serde_json::json!({
                    "role": role,
                    "content": content_value,
                });

                // 工具调用
                if let Some(tool_calls) = &msg.tool_calls {
                    let openai_tool_calls: Vec<Value> = tool_calls
                        .iter()
                        .map(|tc| {
                            serde_json::json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments.to_string(),
                                }
                            })
                        })
                        .collect();
                    obj["tool_calls"] = serde_json::json!(openai_tool_calls);
                }

                // 工具调用 ID (tool 角色)
                if let Some(tool_call_id) = &msg.tool_call_id {
                    obj["tool_call_id"] = serde_json::json!(tool_call_id);
                }

                // reasoning_content 回传 (DeepSeek R1 等需要)
                if let Some(rc) = &msg.reasoning_content {
                    obj["reasoning_content"] = serde_json::json!(rc);
                }

                obj
            })
            .collect()
    }

    /// 将统一工具定义转为 OpenAI 格式
    fn convert_tools(&self, tools: Vec<ToolDefinition>) -> Option<Vec<Value>> {
        if tools.is_empty() {
            return None;
        }
        Some(
            tools
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters,
                        }
                    })
                })
                .collect(),
        )
    }
}

#[async_trait]
impl LlmProvider for OpenAICompatProvider {
    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAICompat
    }

    async fn stream_chat(
        &self,
        messages: Vec<UnifiedMessage>,
        tools: Vec<ToolDefinition>,
        model: &str,
        channel: Channel<StreamChunk>,
    ) -> Result<(), ProviderError> {
        let openai_messages = self.convert_messages(messages);
        let openai_tools = self.convert_tools(tools);

        let mut request_body = serde_json::json!({
            "model": model,
            "messages": openai_messages,
            "stream": true,
        });

        if let Some(tools) = openai_tools {
            request_body["tools"] = serde_json::json!(tools);
        }

        let url = format!("{}/chat/completions", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        // 检查 HTTP 状态码
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::ApiError(format!(
                "HTTP {}: {}",
                status, body
            )));
        }

        // 解析 SSE 流
        let mut stream = response.bytes_stream();
        let mut sse_parser = SseParser::new();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            let text = String::from_utf8_lossy(&chunk);
            let events = sse_parser.feed(&text);

            for lines in events {
                for line in lines {
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            let _ = channel.send(StreamChunk::Done {
                                finish_reason: Some("stop".to_string()),
                            });
                            return Ok(());
                        }

                        // 解析 JSON 块
                        if let Ok(parsed) = serde_json::from_str::<Value>(data) {
                            if let Some(choices) = parsed["choices"].as_array() {
                                for choice in choices {
                                    let delta = &choice["delta"];

                                    // 思考链增量 (DeepSeek R1 / Qwen QwQ 等模型的 reasoning_content)
                                    if let Some(reasoning) = delta["reasoning_content"].as_str() {
                                        if !reasoning.is_empty() {
                                            let _ = channel.send(StreamChunk::ThinkingDelta {
                                                delta: reasoning.to_string(),
                                            });
                                        }
                                    }

                                    // 文本增量
                                    if let Some(content) = delta["content"].as_str() {
                                        if !content.is_empty() {
                                            let _ = channel.send(StreamChunk::TextDelta {
                                                delta: content.to_string(),
                                            });
                                        }
                                    }

                                    // 工具调用增量
                                    if let Some(tool_calls) = delta["tool_calls"].as_array() {
                                        for tc in tool_calls {
                                            let _index = tc["index"].as_u64().unwrap_or(0);
                                            let id = tc["id"].as_str();
                                            let func_name =
                                                tc["function"]["name"].as_str();
                                            let func_args =
                                                tc["function"]["arguments"].as_str();

                                            // 工具调用开始
                                            if let Some(id) = id {
                                                let _ = channel.send(
                                                    StreamChunk::ToolCallStart {
                                                        id: id.to_string(),
                                                        name: func_name
                                                            .unwrap_or("")
                                                            .to_string(),
                                                    },
                                                );
                                            }

                                            // 工具参数增量
                                            if let Some(args) = func_args {
                                                if !args.is_empty() {
                                                    let _ = channel.send(
                                                        StreamChunk::ToolCallDelta {
                                                            id: id.unwrap_or("")
                                                                .to_string(),
                                                            arguments_delta: args
                                                                .to_string(),
                                                        },
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    // 检查是否结束
                                    if let Some(finish_reason) =
                                        choice["finish_reason"].as_str()
                                    {
                                        if !finish_reason.is_empty() {
                                            // 工具调用结束
                                            if finish_reason == "tool_calls" {
                                                // 标记工具调用完成
                                                if let Some(tool_calls) =
                                                    delta["tool_calls"].as_array()
                                                {
                                                    for tc in tool_calls {
                                                        if let Some(id) = tc["id"].as_str()
                                                        {
                                                            let _ = channel.send(
                                                                StreamChunk::ToolCallEnd {
                                                                    id: id.to_string(),
                                                                },
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // 处理错误响应
                            if let Some(error) = parsed["error"].as_object() {
                                let msg = error["message"]
                                    .as_str()
                                    .unwrap_or("未知错误");
                                let _ =
                                    channel.send(StreamChunk::Error {
                                        message: msg.to_string(),
                                    });
                                return Err(ProviderError::ApiError(msg.to_string()));
                            }
                        }
                    }
                }
            }
        }

        let _ = channel.send(StreamChunk::ThinkingDone {});
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
        let openai_messages = self.convert_messages(messages);

        let request_body = serde_json::json!({
            "model": model,
            "messages": openai_messages,
            "stream": false,
        });

        let url = format!("{}/chat/completions", self.api_base);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
        Ok(result["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }

    async fn chat_sync_with_tools(
        &self,
        messages: Vec<UnifiedMessage>,
        tools: &[ToolDefinition],
        model: &str,
    ) -> Result<ChatSyncResult, ProviderError> {
        let openai_messages = self.convert_messages(messages);
        let openai_tools = self.convert_tools(tools.to_vec());

        let mut request_body = serde_json::json!({
            "model": model,
            "messages": openai_messages,
            "stream": false,
        });

        if let Some(ot) = openai_tools {
            request_body["tools"] = serde_json::json!(ot);
            request_body["tool_choice"] = serde_json::json!("auto");
        }

        let url = format!("{}/chat/completions", self.api_base);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
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
        let message = &result["choices"][0]["message"];

        // 检查是否有工具调用
        if let Some(tool_calls) = message["tool_calls"].as_array() {
            if !tool_calls.is_empty() {
                let calls: Vec<ToolCall> = tool_calls
                    .iter()
                    .filter_map(|tc| {
                        Some(ToolCall {
                            id: tc["id"].as_str()?.to_string(),
                            name: tc["function"]["name"].as_str()?.to_string(),
                            arguments: tc["function"]["arguments"]
                                .as_str()
                                .and_then(|a| serde_json::from_str(a).ok())
                                .unwrap_or(serde_json::json!({})),
                        })
                    })
                    .collect();

                // 保留 reasoning_content（DeepSeek R1 等需要回传）
                let reasoning = message["reasoning_content"].as_str().map(|s| s.to_string());

                return Ok(ChatSyncResult::ToolCalls { calls, reasoning });
            }
        }

        // 返回文本内容
        let content = message["content"].as_str().unwrap_or("").to_string();
        Ok(ChatSyncResult::Content(content))
    }
}

/// 读取文件并转为 base64（公开给其他模块使用）
pub fn read_file_base64(path: &str) -> Result<String, String> {
    read_file_as_base64(path)
}

fn read_file_as_base64(path: &str) -> Result<String, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("无法读取文件: {}", e))?;
    let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| format!("读取失败: {}", e))?;
    if buffer.len() > 20 * 1024 * 1024 {
        return Err("图片文件过大（超过20MB）".into());
    }
    Ok(format!("data:{};base64,{}", mime, base64_encode(&buffer)))
}

fn base64_encode(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
}
