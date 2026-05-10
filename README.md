# claude-stream

[![crates.io](https://img.shields.io/crates/v/claude-stream.svg)](https://crates.io/crates/claude-stream)
[![docs.rs](https://docs.rs/claude-stream/badge.svg)](https://docs.rs/claude-stream)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

Parse Anthropic's Server-Sent Events stream into typed events. No SDK dependency, no async runtime lock-in — feed it bytes, drain typed events.

```toml
[dependencies]
claude-stream = "0.1"
```

## Why

There is no official Rust SDK for Anthropic. Community SDKs exist but couple you to a particular HTTP client + async runtime. `claude-stream` is the smallest possible primitive: an incremental SSE parser that knows the Anthropic event vocabulary (`message_start`, `content_block_delta`, `tool_use`, `thinking`, `ping`, …) and produces a typed `Event` enum. Wire it to whatever transport you already have.

## Quick start

```rust
use claude_stream::{Event, EventParser, Delta};

let mut parser = EventParser::new();

// As bytes arrive (sync or async — your call):
loop {
    let chunk: &[u8] = your_transport_read().await;
    if chunk.is_empty() { break; }
    parser.feed(chunk);

    while let Some(event) = parser.next_event().unwrap() {
        match event {
            Event::MessageStart { message } => println!("started {}", message.id),
            Event::ContentBlockDelta { delta: Delta::TextDelta { text }, .. } => {
                print!("{text}");  // streaming text
            }
            Event::MessageStop => break,
            _ => {}
        }
    }
}
# fn your_transport_read() -> std::pin::Pin<Box<dyn std::future::Future<Output = &'static [u8]> + Send>> { Box::pin(async { &[][..] }) }
```

For the simple "I have the full body in a String" case:

```rust
use claude_stream::{Event, EventParser};

let body: &str = /* full SSE response */ "";
let events = EventParser::parse_all(body);
```

## Events covered

- `message_start` — message shell + initial usage
- `content_block_start` — text / tool_use / thinking / server_tool_use
- `content_block_delta` — text_delta / input_json_delta / thinking_delta / signature_delta
- `content_block_stop`
- `message_delta` — stop_reason, cumulative usage
- `message_stop`
- `ping`
- `error`

Unknown event types fall through to `Event::Unknown` so the parser never panics on a new variant Anthropic adds.

## What it doesn't do

- Doesn't make HTTP requests. Use `reqwest`, `hyper`, or whatever you have.
- Doesn't reassemble streaming `tool_use.input` partial JSON into a final `serde_json::Value`. Concatenate the `partial_json` strings yourself, then `serde_json::from_str` once the block stops.
- Doesn't validate against any schema. Forward-compat over strictness.

## License

MIT
