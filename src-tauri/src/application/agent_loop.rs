use std::sync::Arc;
use crate::core::models::{Thread, Turn, TurnItem, Message, MessageRole, TokenUsage, TurnStatus};
use crate::ports::{ModelClient, ToolHost, ThreadStore, EventBus, StreamEvent, ModelRequest, StreamChunk};
use tokio::sync::Mutex;

pub struct AgentLoop {
    model_client: Arc<dyn ModelClient>,
    tool_host: Arc<dyn ToolHost>,
    thread_store: Arc<dyn ThreadStore>,
    event_bus: Arc<dyn EventBus>,
}

impl AgentLoop {
    pub fn new(
        model_client: Arc<dyn ModelClient>,
        tool_host: Arc<dyn ToolHost>,
        thread_store: Arc<dyn ThreadStore>,
        event_bus: Arc<dyn EventBus>,
    ) -> Self {
        Self {
            model_client,
            tool_host,
            thread_store,
            event_bus,
        }
    }

    pub async fn run_turn(
        &self,
        thread_id: String,
        user_message: String,
        model: String,
    ) -> Result<Turn, String> {
        let mut turn = Turn::new(thread_id.clone(), model.clone());

        turn.add_item(TurnItem::UserMessage {
            text: user_message.clone(),
            attachments: vec![],
        });

        self.event_bus.emit(&thread_id, StreamEvent::TurnStarted {
            turn_id: turn.id.clone(),
            model: model.clone(),
        });

        let thread = self.thread_store.get_thread(&thread_id).await
            .map_err(|e| e.to_string())?;

        let mut context = self.build_context(&thread, &turn).await?;

        const MAX_ITERATIONS: usize = 15;
        let mut iteration = 0;

        while iteration < MAX_ITERATIONS {
            iteration += 1;

            let tools = self.tool_host.list_tools().await;
            let request = ModelRequest {
                messages: context.clone(),
                model: model.clone(),
                tools,
                max_tokens: Some(4096),
                temperature: Some(0.7),
            };

            let mut stream = self.model_client.stream_completion(request).await
                .map_err(|e| e.to_string())?;

            let mut accumulated_text = String::new();
            let mut accumulated_reasoning = String::new();
            let mut tool_calls = Vec::new();
            let mut current_tool_call: Option<(String, String, String)> = None;

            use futures::StreamExt;
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.map_err(|e| e.to_string())?;

                match chunk {
                    StreamChunk::TextDelta { text } => {
                        accumulated_text.push_str(&text);
                        self.event_bus.emit(&thread_id, StreamEvent::TextDelta { text });
                    }
                    StreamChunk::ReasoningDelta { text } => {
                        accumulated_reasoning.push_str(&text);
                        self.event_bus.emit(&thread_id, StreamEvent::ReasoningDelta { text });
                    }
                    StreamChunk::ToolCallStart { id, name } => {
                        current_tool_call = Some((id.clone(), name.clone(), String::new()));
                        self.event_bus.emit(&thread_id, StreamEvent::ToolCallStarted { id, name });
                    }
                    StreamChunk::ToolCallArgsDelta { id, args_json } => {
                        if let Some((ref call_id, _, ref mut args)) = current_tool_call {
                            if call_id == &id {
                                args.push_str(&args_json);
                            }
                        }
                        self.event_bus.emit(&thread_id, StreamEvent::ToolCallArgsDelta { id, args_json });
                    }
                    StreamChunk::ToolCallComplete { id } => {
                        if let Some((call_id, name, args)) = current_tool_call.take() {
                            if call_id == id {
                                let args_value: serde_json::Value = serde_json::from_str(&args)
                                    .unwrap_or(serde_json::json!({}));
                                tool_calls.push((call_id, name, args_value));
                            }
                        }
                        self.event_bus.emit(&thread_id, StreamEvent::ToolCallComplete { id });
                    }
                    StreamChunk::Done { usage } => {
                        turn.usage.prompt_tokens += usage.prompt_tokens;
                        turn.usage.completion_tokens += usage.completion_tokens;
                        turn.usage.cache_read_tokens += usage.cache_read_tokens;
                        turn.usage.cache_creation_tokens += usage.cache_creation_tokens;
                        turn.usage.total_tokens += usage.total_tokens;
                    }
                }
            }

            if !accumulated_reasoning.is_empty() {
                turn.add_item(TurnItem::AssistantReasoning {
                    content: accumulated_reasoning.clone(),
                });
            }

            if !accumulated_text.is_empty() {
                turn.add_item(TurnItem::AssistantText {
                    text: accumulated_text.clone(),
                });
                context.push(Message::assistant(accumulated_text.clone()));
            }

            if tool_calls.is_empty() {
                break;
            }

            for (call_id, name, args) in tool_calls {
                turn.add_item(TurnItem::ToolCall {
                    id: call_id.clone(),
                    name: name.clone(),
                    args: args.clone(),
                });

                let tool_call = crate::core::models::ToolCall {
                    id: call_id.clone(),
                    name: name.clone(),
                    arguments: args,
                };

                let result = self.tool_host.execute(&tool_call).await;

                match result {
                    Ok(tool_result) => {
                        turn.add_item(TurnItem::ToolResult {
                            call_id: call_id.clone(),
                            result: tool_result.output.clone(),
                            error: tool_result.error.clone(),
                        });

                        self.event_bus.emit(&thread_id, StreamEvent::ToolResultReceived {
                            call_id: call_id.clone(),
                            result: tool_result.output.clone(),
                            error: tool_result.error.clone(),
                        });

                        context.push(Message {
                            role: MessageRole::User,
                            content: vec![crate::core::models::ContentPart::ToolResult {
                                tool_use_id: call_id,
                                content: tool_result.output,
                                is_error: tool_result.error.is_some(),
                            }],
                        });
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        turn.add_item(TurnItem::ToolResult {
                            call_id: call_id.clone(),
                            result: String::new(),
                            error: Some(error_msg.clone()),
                        });

                        self.event_bus.emit(&thread_id, StreamEvent::ToolResultReceived {
                            call_id: call_id.clone(),
                            result: String::new(),
                            error: Some(error_msg.clone()),
                        });

                        context.push(Message {
                            role: MessageRole::User,
                            content: vec![crate::core::models::ContentPart::ToolResult {
                                tool_use_id: call_id,
                                content: error_msg,
                                is_error: true,
                            }],
                        });
                    }
                }
            }
        }

        turn.complete(turn.usage.clone());
        self.thread_store.save_turn(&turn).await.map_err(|e| e.to_string())?;

        self.event_bus.emit(&thread_id, StreamEvent::TurnCompleted {
            usage: turn.usage.clone(),
        });

        Ok(turn)
    }

    async fn build_context(&self, thread: &Thread, turn: &Turn) -> Result<Vec<Message>, String> {
        let mut messages = Vec::new();

        messages.push(Message::system(
            "You are a helpful AI assistant with access to tools.".to_string()
        ));

        if let Some(workspace) = &thread.workspace_path {
            messages.push(Message::system(
                format!("Current workspace: {}", workspace.display())
            ));
        }

        let recent_turns = self.thread_store.list_turns(&thread.id, 20).await
            .map_err(|e| e.to_string())?;

        for past_turn in recent_turns.iter().rev() {
            for item in &past_turn.items {
                match item {
                    TurnItem::UserMessage { text, .. } => {
                        messages.push(Message::user(text.clone()));
                    }
                    TurnItem::AssistantText { text } => {
                        messages.push(Message::assistant(text.clone()));
                    }
                    _ => {}
                }
            }
        }

        if let Some(TurnItem::UserMessage { text, .. }) = turn.items.first() {
            messages.push(Message::user(text.clone()));
        }

        Ok(messages)
    }
}
