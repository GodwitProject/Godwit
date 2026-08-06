use futures::{Stream, StreamExt};
use godwit_core::{FunctionCall, ToolCall};
use godwit_providers::{ProviderError, SseEvent};
use godwit_providers::sse_egress::CanonicalEvent;
use serde_json::Value;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::{proxy::resolve_tool_calls, state::AppState};

#[derive(Debug, Clone)]
pub struct CompleteToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Default)]
pub struct ToolCallBuffer {
    pub current_tool: Option<CurrentToolState>,
    pub accumulated_tools: Vec<CompleteToolCall>,
}

#[derive(Debug, Clone)]
pub struct CurrentToolState {
    pub index: usize,
    pub id: String,
    pub name: String,
    pub arguments: String,
}

impl ToolCallBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_delta(&mut self, delta: &str) {
        let parsed: Value = match serde_json::from_str(delta) {
            Ok(v) => v,
            Err(_) => return,
        };

        let index = parsed
            .get("index")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let id = parsed
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let name = parsed
            .get("function")
            .and_then(|f| f.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let args = parsed
            .get("function")
            .and_then(|f| f.get("arguments"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(ref mut state) = self.current_tool {
            if state.index == index {
                if !name.is_empty() {
                    state.name = name;
                }
                state.arguments.push_str(&args);
                if !id.is_empty() {
                    state.id = id;
                }
            } else {
                let complete = CompleteToolCall {
                    id: state.id.clone(),
                    r#type: "function".to_string(),
                    function: FunctionCall {
                        name: state.name.clone(),
                        arguments: state.arguments.clone(),
                    },
                };
                self.accumulated_tools.push(complete);
                self.current_tool = Some(CurrentToolState {
                    index,
                    id,
                    name,
                    arguments: args,
                });
            }
        } else {
            self.current_tool = Some(CurrentToolState {
                index,
                id,
                name,
                arguments: args,
            });
        }
    }

    pub fn finish_current_tool(&mut self) -> Option<CompleteToolCall> {
        self.current_tool.take().map(|state| CompleteToolCall {
            id: state.id,
            r#type: "function".to_string(),
            function: FunctionCall {
                name: state.name,
                arguments: state.arguments,
            },
        })
    }

    pub fn has_complete_tool(&self) -> bool {
        self.current_tool
            .as_ref()
            .map(|s| !s.arguments.is_empty() && !s.name.is_empty())
            .unwrap_or(false)
    }

    pub fn finish_all(&mut self) -> Vec<CompleteToolCall> {
        let mut all = std::mem::take(&mut self.accumulated_tools);
        if let Some(complete) = self.finish_current_tool() {
            all.push(complete);
        }
        all
    }
}

pub fn process_streaming_tool_calls(
    state: Arc<AppState>,
    stream: impl Stream<Item = Result<SseEvent, ProviderError>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<SseEvent, ProviderError>> + Send>> {
    let (tx, rx) = mpsc::channel::<Result<SseEvent, ProviderError>>(64);

    tokio::spawn(async move {
        let mut stream = Box::pin(stream);
        let mut buffer = ToolCallBuffer::new();
        let mut tool_call_detected = false;
        let mut finished = false;

        while let Some(event_result) = stream.next().await {
            match event_result {
                Ok(event) => {
                    match parse_canonical_event(&event.data) {
                        CanonicalEvent::ToolCall(packed) => {
                            tool_call_detected = true;
                            buffer.push_delta(&packed);
                            let _ = tx.send(Ok(event)).await;
                        }
                        CanonicalEvent::Finish { .. } => {
                            finished = true;
                            let _ = tx.send(Ok(event)).await;

                            if tool_call_detected {
                                let all_tools = buffer.finish_all();
                                for complete_tool in all_tools {
                                    if !complete_tool.function.name.is_empty() {
                                        let tool_calls = vec![ToolCall {
                                            id: complete_tool.id,
                                            r#type: complete_tool.r#type,
                                            function: complete_tool.function,
                                        }];

                                        let tool_results = resolve_tool_calls(&state, &tool_calls).await;

                                        for result_msg in tool_results {
                                            let result_event = SseEvent {
                                                data: serde_json::json!({
                                                    "type": "tool_result",
                                                    "message": result_msg
                                                }).to_string(),
                                            };
                                            let _ = tx.send(Ok(result_event)).await;
                                        }
                                    }
                                }
                            }
                        }
                        _ => {
                            let _ = tx.send(Ok(event)).await;
                        }
                    }
                }
                Err(e) => {
                    let error_event = SseEvent {
                        data: serde_json::json!({
                            "type": "error",
                            "message": e.to_string()
                        }).to_string(),
                    };
                    let _ = tx.send(Ok(error_event)).await;
                    break;
                }
            }
        }

        drop(tx);
    });

    Box::pin(ReceiverStream::new(rx))
}

fn parse_canonical_event(data: &str) -> CanonicalEvent {
    godwit_providers::sse_egress::parse_canonical_event(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::stream;

    #[test]
    fn buffer_accumulates_tool_call_delta() {
        let mut buffer = ToolCallBuffer::new();
        let delta1 = r#"{"index":0,"id":"call_1","function":{"name":"get_"}}"#;
        let delta2 = r#"{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":"{}"}}"#;
        
        buffer.push_delta(delta1);
        assert!(buffer.current_tool.is_some());
        
        buffer.push_delta(delta2);
        let complete = buffer.finish_current_tool();
        assert!(complete.is_some());
        assert_eq!(complete.unwrap().function.name, "get_weather");
    }

    #[test]
    fn buffer_returns_none_on_incomplete_json() {
        let mut buffer = ToolCallBuffer::new();
        let incomplete = r#"{"index":0,"id":"call_1","function":{"name":"get"#;
        
        buffer.push_delta(incomplete);
        let complete = buffer.finish_current_tool();
        assert!(complete.is_none());
    }

    #[test]
    fn buffer_handles_multiple_tools() {
        let mut buffer = ToolCallBuffer::new();
        let tool1 = r#"{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":"{}"}}"#;
        let tool2 = r#"{"index":1,"id":"call_2","function":{"name":"search","arguments":"{}"}}"#;
        
        buffer.push_delta(tool1);
        buffer.push_delta(tool2);
        
        assert_eq!(buffer.accumulated_tools.len(), 1);
        assert_eq!(buffer.accumulated_tools[0].function.name, "get_weather");
        
        let complete = buffer.finish_current_tool();
        assert!(complete.is_some());
        assert_eq!(complete.as_ref().unwrap().function.name, "search");
    }

    #[test]
    fn buffer_accumulates_arguments_across_deltas() {
        let mut buffer = ToolCallBuffer::new();
        let delta1 = r#"{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":"{\"ci"}}"#;
        let delta2 = r#"{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":"ty\":\"Paris\"}"}}"#;
        
        buffer.push_delta(delta1);
        buffer.push_delta(delta2);
        
        let complete = buffer.finish_current_tool();
        assert!(complete.is_some());
        assert!(complete.as_ref().unwrap().function.arguments.contains("Paris"));
    }

    #[test]
    fn buffer_finish_all_returns_all_tools() {
        let mut buffer = ToolCallBuffer::new();
        let tool1 = r#"{"index":0,"id":"call_1","function":{"name":"get_weather","arguments":"{}"}}"#;
        let tool2 = r#"{"index":1,"id":"call_2","function":{"name":"search","arguments":"{}"}}"#;
        
        buffer.push_delta(tool1);
        buffer.push_delta(tool2);
        
        let all = buffer.finish_all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].function.name, "get_weather");
        assert_eq!(all[1].function.name, "search");
    }
}
