use criterion::{criterion_group, criterion_main, Criterion};
use godwit_core::{ChatCompletionRequest, ChatContent, ChatMessage};
use godwit_providers::anthropic::AnthropicChatRequest;

fn bench_anthropic_mapping(c: &mut Criterion) {
    let req = ChatCompletionRequest {
        model: "claude-sonnet".to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: ChatContent::text("You are helpful"),
                name: None,
                ..Default::default()
            },
            ChatMessage {
                role: "user".to_string(),
                content: ChatContent::text("Hello"),
                name: None,
                ..Default::default()
            },
        ],
        stream: Some(false),
        temperature: Some(0.7),
        max_tokens: Some(100),
        ..Default::default()
    };
    c.bench_function("openai_to_anthropic_mapping", |b| {
        b.iter(|| AnthropicChatRequest::from_chat_request(req.clone(), "claude-3-5-sonnet".to_string()))
    });
}

criterion_group!(benches, bench_anthropic_mapping);
criterion_main!(benches);
