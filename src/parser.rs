use crate::event::{Event, ParseError};

/// Incremental SSE parser for Anthropic's `messages` stream.
///
/// Stateful: feed bytes as they arrive, drain ready events with
/// [`next_event`](Self::next_event). Safe to call `feed` after every
/// network read of any size, including mid-event.
pub struct EventParser {
    /// Bytes received but not yet split into complete events.
    buf: String,
    /// Events parsed but not yet handed to the caller.
    pending: std::collections::VecDeque<Event>,
}

impl EventParser {
    /// Construct an empty parser.
    pub fn new() -> Self {
        Self {
            buf: String::new(),
            pending: std::collections::VecDeque::new(),
        }
    }

    /// Append `bytes` to the parser's buffer and process any complete events.
    ///
    /// Non-UTF-8 bytes are dropped (Anthropic's SSE is always UTF-8 in
    /// practice). Returns the number of new events parsed and queued.
    pub fn feed(&mut self, bytes: &[u8]) -> usize {
        self.buf.push_str(&String::from_utf8_lossy(bytes));
        let mut new_events = 0;
        // Each event is terminated by a blank line ("\n\n").
        while let Some(idx) = self.buf.find("\n\n") {
            let raw = self.buf[..idx].to_string();
            self.buf.drain(..idx + 2);
            if let Some(event) = parse_event_block(&raw) {
                self.pending.push_back(event);
                new_events += 1;
            }
        }
        new_events
    }

    /// Pull the next ready event, or `Ok(None)` if the buffer is empty.
    pub fn next_event(&mut self) -> Result<Option<Event>, ParseError> {
        Ok(self.pending.pop_front())
    }

    /// Drain all currently ready events.
    pub fn drain(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.pending).into_iter().collect()
    }

    /// Convenience: parse a complete SSE response body all at once.
    pub fn parse_all(text: &str) -> Vec<Event> {
        let mut p = Self::new();
        p.feed(text.as_bytes());
        p.drain()
    }
}

impl Default for EventParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse one event block: a sequence of `event:` / `data:` lines, terminated
/// by a blank line. We only care about the `data:` payload (it carries the
/// JSON whose `type` field tells us the event).
fn parse_event_block(raw: &str) -> Option<Event> {
    let mut data = String::new();
    for line in raw.split('\n') {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("data:") {
            // SSE allows multiple `data:` lines for one event; concatenate with newline.
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest.strip_prefix(' ').unwrap_or(rest));
        }
        // Ignore "event:", "id:", "retry:" lines — the JSON `type` field is
        // the source of truth for our discriminant.
    }
    if data.is_empty() {
        return None;
    }
    serde_json::from_str(&data).ok()
}
