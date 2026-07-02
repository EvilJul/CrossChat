use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

use crate::ports::model_client::{ModelClient, ModelError, ModelRequest, ModelResult, StreamChunk};
use crate::core::models::{ContentPart, Message, MessageRole, TokenUsage};

#[derive(Clone)]
pub struct OpenAIClient {
    client: Client,
    api_key: String,
    base_url: String,
}

impl OpenAIClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: OpenAIContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum OpenAIContent {
    Text(String),
    Parts(Vec<OpenAIContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OpenAIContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
}

#[derive(Serialize)]
struct ImageUrl {
    url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Serialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunction,
}

#[derive(Serialize)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIChoice>,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    delta: OpenAIDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIToolCallDelta>>,
}

#[derive(Deserialize)]
struct OpenAIToolCallDelta {
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAIFunctionCallDelta>,
}

#[derive(Deserialize)]
struct OpenAIFunctionCallDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

fn convert_message(msg: &Message) -> OpenAIMessage {
    let role = match msg.role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    };

    let has_multipart = msg.content.iter().any(|p| !matches!(p, ContentPart::Text { .. }));

    let (content, tool_calls, tool_call_id) = if msg.role == MessageRole::Assistant {
        let mut tool_calls_vec = Vec::new();
        let mut text_parts = Vec::new();

        for part in &msg.content {
            match part {
                ContentPart::ToolUse { id, name, input } => {
                    tool_calls_vec.push(OpenAIToolCall {
                        id: id.clone(),
                        tool_type: "function".to_string(),
                        function: OpenAIFunctionCall {
                            name: name.clone(),
                            arguments: input.to_string(),
                        },
                    });
                }
                ContentPart::Text { text } => text_parts.push(text.clone()),
                _ => {}
            }
        }

        let content = if text_parts.is_empty() {
            OpenAIContent::Text(String::new())
        } else {
            OpenAIContent::Text(text_parts.join("\n"))
        };

        (content, if tool_calls_vec.is_empty() { None } else { Some(tool_calls_vec) }, None)
    } else if matches!(msg.content.first(), Some(ContentPart::ToolResult { .. })) {
        if let Some(ContentPart::ToolResult { tool_use_id, content, .. }) = msg.content.first() {
            (OpenAIContent::Text(content.clone()), None, Some(tool_use_id.clone()))
        } else {
            (OpenAIContent::Text(String::new()), None, None)
        }
    } else if has_multipart {
        let parts = msg.content.iter().map(|p| match p {
            ContentPart::Text { text } => OpenAIContentPart::Text { text: text.clone() },
            ContentPart::Image { url, detail } => OpenAIContentPart::ImageUrl {
                image_url: ImageUrl { url: url.clone(), detail: detail.clone() },
            },
            _ => OpenAIContentPart::Text { text: String::new() },
        }).collect();
        (OpenAIContent::Parts(parts), None, None)
    } else {
        let text = msg.content.iter()
            .filter_map(|p| if let ContentPart::Text { text } = p { Some(text.as_str()) } else { None })
            .collect::<Vec<_>>()
            .join("\n");
        (OpenAIContent::Text(text), None, None)
    };

    let mut result = OpenAIMessage { role: role.to_string(), content, tool_calls: None, tool_call_id: None };
    if tool_calls.is_some() {
        result.tool_calls = tool_calls;
    }
    if tool_call_id.is_some() {
        result.tool_call_id = tool_call_id;
        result.role = "tool".to_string();
    }
    result
}

#[async_trait]
impl ModelClient for OpenAIClient {
    async fn stream_completion(
        &self,
        request: ModelRequest,
    ) -> ModelResult<Box<dyn Stream<Item = ModelResult<StreamChunk>> + Send + Unpin>> {
        let messages: Vec<OpenAIMessage> = request.messages.iter().map(convert_message).collect();

        let tools = if request.tools.is_empty() {
            None
        } else {
            Some(request.tools.iter().map(|t| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: t.parameters.clone(),
                },
            }).collect())
        };

        let req = OpenAIRequest {
            model: request.model,
            messages,
            tools,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            stream: true,
        };

        let response = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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
        let mut tool_calls: std::collections::HashMap<usize, (Option<String>, Option<String>, String)> = std::collections::HashMap::new();

        let chunk_stream = stream.map(move |chunk_result| {
            let chunk = chunk_result.map_err(|e| ModelError::StreamError(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);

            let mut results = Vec::new();

            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || !line.starts_with("data: ") {
                    continue;
                }

                let data = &line[6..];
                if data == "[DONE]" {
                    continue;
                }

                let parsed: OpenAIStreamResponse = serde_json::from_str(data)
                    .map_err(|e| ModelError::InvalidResponse(e.to_string()))?;

                if let Some(usage) = parsed.usage {
                    results.push(Ok(StreamChunk::Done {
                        usage: TokenUsage {
                            prompt_tokens: usage.prompt_tokens,
                            completion_tokens: usage.completion_tokens,
                            total_tokens: usage.total_tokens,
                            cache_read_tokens: 0,
                            cache_creation_tokens: 0,
                        },
                    }));
                    continue;
                }

                for choice in &parsed.choices {
                    if let Some(content) = &choice.delta.content {
                        if !content.is_empty() {
                            results.push(Ok(StreamChunk::TextDelta { text: content.clone() }));
                        }
                    }

                    if let Some(tool_calls_delta) = &choice.delta.tool_calls {
                        for tc_delta in tool_calls_delta {
                            let entry = tool_calls.entry(tc_delta.index).or_insert((None, None, String::new()));

                            if let Some(id) = &tc_delta.id {
                                entry.0 = Some(id.clone());
                            }

                            if let Some(func) = &tc_delta.function {
                                if let Some(name) = &func.name {
                                    entry.1 = Some(name.clone());
                                    if let (Some(id), Some(name)) = (&entry.0, &entry.1) {
                                        results.push(Ok(StreamChunk::ToolCallStart {
                                            id: id.clone(),
                                            name: name.clone(),
                                        }));
                                    }
                                }
                                if let Some(args) = &func.arguments {
                                    entry.2.push_str(args);
                                    if let Some(id) = &entry.0 {
                                        results.push(Ok(StreamChunk::ToolCallArgsDelta {
                                            id: id.clone(),
                                            args_json: args.clone(),
                                        }));
                                    }
                                }
                            }
                        }
                    }

                    if choice.finish_reason.is_some() {
                        for (_, (id_opt, _, _)) in &tool_calls {
                            if let Some(id) = id_opt {
                                results.push(Ok(StreamChunk::ToolCallComplete { id: id.clone() }));
                            }
                        }
                    }
                }
            }

            if results.is_empty() {
                return Ok(StreamChunk::TextDelta { text: String::new() });
            }

            if results.len() == 1 {
                return results.into_iter().next().unwrap();
            }

            results.into_iter().next().unwrap()
        });

        Ok(Box::new(Box::pin(chunk_stream)))
    }

    fn supports_vision(&self) -> bool {
        true
    }
}
