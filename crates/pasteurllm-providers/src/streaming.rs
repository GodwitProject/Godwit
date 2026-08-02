use crate::SseEvent;

pub fn parse_sse_events(chunk: &str) -> Vec<SseEvent> {
    let mut events = Vec::new();
    for line in chunk.lines() {
        let line = line.trim();
        if line.is_empty() || line == ":" {
            continue;
        }
        if let Some(data) = line.strip_prefix("data: ") {
            if data == "[DONE]" {
                continue;
            }
            events.push(SseEvent {
                data: data.to_string(),
            });
        }
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_openai_sse_chunk() {
        let line = "data: {\"id\":\"1\",\"object\":\"chat.completion.chunk\"}\n\n";
        let events = parse_sse_events(line);
        assert_eq!(events.len(), 1);
        assert!(events[0].data.contains("chat.completion.chunk"));
    }

    #[test]
    fn ignores_sse_done() {
        let line = "data: [DONE]\n\n";
        let events = parse_sse_events(line);
        assert!(events.is_empty());
    }
}
