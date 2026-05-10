use claude_stream::{ContentBlock, Delta, Event, EventParser};

const SAMPLE: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-20250514\"}}\n\
\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\", world\"}}\n\
\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\
\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":12}}\n\
\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\
\n";

#[test]
fn parse_all_returns_full_event_sequence() {
    let events = EventParser::parse_all(SAMPLE);
    assert_eq!(events.len(), 7);
    assert!(matches!(events[0], Event::MessageStart { .. }));
    assert!(matches!(events[1], Event::ContentBlockStart { .. }));
    assert!(matches!(events[2], Event::ContentBlockDelta { .. }));
    assert!(matches!(events[6], Event::MessageStop));
}

#[test]
fn message_start_carries_id_and_model() {
    let events = EventParser::parse_all(SAMPLE);
    if let Event::MessageStart { message } = &events[0] {
        assert_eq!(message.id, "msg_1");
        assert_eq!(message.model.as_deref(), Some("claude-sonnet-4-20250514"));
    } else {
        panic!("expected MessageStart");
    }
}

#[test]
fn text_deltas_carry_text() {
    let events = EventParser::parse_all(SAMPLE);
    let mut texts = Vec::new();
    for e in &events {
        if let Event::ContentBlockDelta { delta: Delta::TextDelta { text }, .. } = e {
            texts.push(text.clone());
        }
    }
    assert_eq!(texts, vec!["Hello".to_string(), ", world".to_string()]);
}

#[test]
fn message_delta_includes_usage_and_stop_reason() {
    let events = EventParser::parse_all(SAMPLE);
    if let Event::MessageDelta { delta, usage } = &events[5] {
        assert_eq!(delta.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(usage.output_tokens, Some(12));
    } else {
        panic!("expected MessageDelta");
    }
}

#[test]
fn incremental_feed_handles_split_chunks() {
    let mut p = EventParser::new();
    let halfway = SAMPLE.len() / 2;
    p.feed(&SAMPLE.as_bytes()[..halfway]);
    p.feed(&SAMPLE.as_bytes()[halfway..]);
    let events = p.drain();
    assert_eq!(events.len(), 7);
}

#[test]
fn incremental_feed_handles_byte_at_a_time() {
    let mut p = EventParser::new();
    for byte in SAMPLE.as_bytes() {
        p.feed(&[*byte]);
    }
    let events = p.drain();
    assert_eq!(events.len(), 7);
}

#[test]
fn ping_event_parses() {
    let events = EventParser::parse_all("event: ping\ndata: {\"type\":\"ping\"}\n\n");
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], Event::Ping));
}

#[test]
fn unknown_event_type_falls_through() {
    let events =
        EventParser::parse_all("event: future_event\ndata: {\"type\":\"future_event\"}\n\n");
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], Event::Unknown));
}

#[test]
fn tool_use_block_with_input_json_deltas() {
    let s = "event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"get_weather\",\"input\":{}}}\n\
\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"city\\\":\"}}\n\
\n";
    let events = EventParser::parse_all(s);
    assert_eq!(events.len(), 2);
    if let Event::ContentBlockStart { content_block, .. } = &events[0] {
        match content_block {
            ContentBlock::ToolUse { id, name, .. } => {
                assert_eq!(id, "toolu_1");
                assert_eq!(name, "get_weather");
            }
            _ => panic!("expected ToolUse"),
        }
    }
    if let Event::ContentBlockDelta { delta, .. } = &events[1] {
        if let Delta::InputJsonDelta { partial_json } = delta {
            assert_eq!(partial_json, "{\"city\":");
        } else {
            panic!("expected InputJsonDelta");
        }
    }
}

#[test]
fn malformed_data_line_is_dropped_quietly() {
    // A `data:` line that's not valid JSON is silently dropped — the
    // stream continues. (Anthropic doesn't actually emit this, but
    // forward-compat).
    let s = "event: junk\ndata: not json\n\nevent: ping\ndata: {\"type\":\"ping\"}\n\n";
    let events = EventParser::parse_all(s);
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], Event::Ping));
}

#[test]
fn empty_input_yields_no_events() {
    assert!(EventParser::parse_all("").is_empty());
}
