use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ports::model_client::{ModelClient, ModelError, ModelRequest, ModelResult, StreamChunk};
use crate::core::models::{ContentPart, Message, MessageRole, TokenUsage};

#[derive(Clone)]
pub struct AnthropicClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl AnthropicClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    Text { text: String },
    Image { source: ImageSource },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String, #[serde(skip_serializing_if = "Option::is_none")] is_error: Option<bool> },
}

#[derive(Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: Value,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    message: Option<AnthropicMessageResponse>,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    content_block: Option<ContentBlockResponse>,
    #[serde(default)]
    delta: Option<DeltaResponse>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicMessageResponse {
    usage: AnthropicUsage,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentBlockResponse {
    Text { text: String },
    Thinking { thinking: String },
    ToolUse { id: String, name: String, input: Value },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DeltaResponse {
    TextDelta { text: String },
    ThinkingDelta { thinking: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
}

fn convert_messages(msgs: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system = None;
    let mut messages = Vec::new();

    for msg in msgs {
        if msg.role == MessageRole::System {
            let text = msg.content.iter()
                .filter_map(|p| if let ContentPart::Text { text } = p { Some(text.as_str()) } else { None })
                .collect::<Vec<_>>()
                .join("\n");
            system = Some(text);
            continue;
        }

        let role = match msg.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => continue,
        };

        let content = msg.content.iter().map(|p| match p {
            ContentPart::Text { text } => AnthropicContentBlock::Text { text: text.clone() },
            ContentPart::Image { url, .. } => {
                let (media_type, data) = if url.starts_with("data:") {
                    let parts: Vec<&str> = url.splitn(2, ',').collect();
                    if parts.len() == 2 {
                        let header = parts[0].trim_start_matches("data:");
                        let media = header.split(';').next().unwrap_or("image/jpeg");
                        (media.to_string(), parts[1].to_string())
                    } else {
                        ("image/jpeg".to_string(), url.clone())
                    }
                } else {
                    ("image/jpeg".to_string(), url.clone())
                };
                AnthropicContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type,
                        data,
                    },
                }
            },
            ContentPart::ToolUse { id, name, input } => {
                AnthropicContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                }
            },
            ContentPart::ToolResult { tool_use_id, content, is_error } => {
                AnthropicContentBlock::ToolResult {
                    tool_use_id: tool_use_id.clone(),
                    content: content.clone(),
                    is_error: if *is_error { Some(true) } else { None },
                }
            },
        }).collect();

        messages.push(AnthropicMessage {
            role: role.to_string(),
            content,
        });
    }

    (system, messages)
}

#[async_trait]
impl ModelClient for AnthropicClient {
    async fn stream_completion(
        &self,
        request: ModelRequest,
    ) -> ModelResult<Box<dyn Stream<Item = ModelResult<StreamChunk>> + Send + Unpin>> {
        let (system, messages) = convert_messages(&request.messages);

        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(request.tools.iter().map(|t| AnthropicTool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.parameters.clone(),
            }).collect())
        };

        let req = AnthropicRequest {
            model: request.model,
            messages,
            system,
            tools,
            max_tokens: request.max_tokens.unwrap_or(4096),
            temperature: request.temperature,
            stream: true,
        };

        let response = self.client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await
            .map_err(|e| ModelError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(if status.as_u16() == 429 {
                ModelError::RateLimitExceeded
            } else {
                ModelError::ApiError(format!("{}: {}", status, body))
            });
        }

        let stream = response.bytes_stream();
        let mut tool_calls: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

        let chunk_stream = stream.map(move |chunk_result| {
            let chunk = chunk_result.map_err(|e| ModelError::StreamError(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);

            for line in text.lines() {
                let line = line.trim();
                if !line.starts_with("data: ") {
                    continue;
                }

                let data = &line[6..];
                let event: AnthropicStreamEvent = serde_json::from_str(data)
                    .map_err(|e| ModelError::InvalidResponse(e.to_string()))?;

                match event.event_type.as_str() {
                    "message_start" => {
                        if let Some(msg) = event.message {
                            return Ok(StreamChunk::Done {
                                usage: TokenUsage {
                                    prompt_tokens: msg.usage.input_tokens,
                                    completion_tokens: msg.usage.output_tokens,
                                    total_tokens: msg.usage.input_tokens + msg.usage.output_tokens,
                                    cache_read_tokens: msg.usage.cache_read_input_tokens.unwrap_or(0),
                                    cache_creation_tokens: msg.usage.cache_creation_input_tokens.unwrap_or(0),
                                },
                            });
                        }
                    }
                    "content_block_start" => {
                        if let Some(block) = event.content_block {
                            match block {
                                ContentBlockResponse::ToolUse { id, name, .. } => {
                                    tool_calls.insert(id.clone(), (name.clone(), String::new()));
                                    return Ok(StreamChunk::ToolCallStart { id, name });
                                }
                                _ => {}
                            }
                        }
                    }
                    "content_block_delta" => {
                        if let Some(delta) = event.delta {
                            match delta {
                                DeltaResponse::TextDelta { text } => {
                                    return Ok(StreamChunk::TextDelta { text });
                                }
                                DeltaResponse::ThinkingDelta { thinking } => {
                                    return Ok(StreamChunk::ReasoningDelta { text: thinking });
                                }
                                DeltaResponse::InputJsonDelta { partial_json } => {
                                    if let Some(index) = event.index {
                                        if let Some((id, _)) = tool_calls.iter().nth(index) {
                                            let id = id.clone();
                                            return Ok(StreamChunk::ToolCallArgsDelta {
                                                id,
                                                args_json: partial_json,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    "content_block_stop" => {
                        if let Some(index) = event.index {
                            if let Some((id, _)) = tool_calls.iter().nth(index) {
                                return Ok(StreamChunk::ToolCallComplete { id: id.clone() });
                            }
                        }
                    }
                    "message_delta" => {
                        if let Some(usage) = event.usage {
                            return Ok(StreamChunk::Done {
                                usage: TokenUsage {
                                    prompt_tokens: usage.input_tokens,
                                    completion_tokens: usage.output_tokens,
                                    total_tokens: usage.input_tokens + usage.output_tokens,
                                    cache_read_tokens: usage.cache_read_input_tokens.unwrap_or(0),
                                    cache_creation_tokens: usage.cache_creation_input_tokens.unwrap_or(0),
                                },
                            });
                        }
                    }
                    _ => {}
                }
            }

            Ok(StreamChunk::TextDelta { text: String::new() })
        });

        Ok(Box::new(Box::pin(chunk_stream)))
    }

    fn supports_reasoning(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }
}
