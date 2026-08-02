use criterion::{criterion_group, criterion_main, Criterion};
use godwit_core::{ChatCompletionRequest, ChatMessage};
use godwit_providers::anthropic::to_anthropic_request;

fn bench_anthropic_mapping(c: &mut Criterion) {
    let req = ChatCompletionRequest {
        model: "claude-sonnet".to_string(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: "You are helpful".to_string() },
            ChatMessage { role: "user".to_string(), content: "Hello".to_string() },
        ],
        stream: Some(false),
        temperature: Some(0.7),
        max_tokens: Some(100),
    };
    c.bench_function("openai_to_anthropic_mapping", |b| {
        b.iter(|| to_anthropic_request(&req))
    });
}

criterion_group!(benches, bench_anthropic_mapping);
criterion_main!(benches);
