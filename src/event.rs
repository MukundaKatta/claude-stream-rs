use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Errors from parsing one event.
#[derive(Debug, Error)]
pub enum ParseError {
    /// JSON in the `data:` line couldn't be decoded.
    #[error("invalid event JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// Missing required field in the event payload.
    #[error("malformed event: {0}")]
    Malformed(String),
}

/// One Anthropic streaming event.
///
/// Mirrors [the `messages` streaming wire format][docs] as of late 2025.
/// Unknown event types fall through to [`Event::Unknown`] so the parser
/// never panics on a new event Anthropic adds.
///
/// [docs]: https://docs.anthropic.com/en/api/messages-streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    /// Sent first; carries the response shell (id, role, model, initial usage).
    MessageStart {
        /// The shell message: id, role, model, etc.
        message: Message,
    },
    /// A new content block is starting (text, tool_use, thinking, etc.).
    ContentBlockStart {
        /// Index in the message's `content` array.
        index: u32,
        /// The (initially empty) content block.
        content_block: ContentBlock,
    },
    /// A delta extending an in-progress content block.
    ContentBlockDelta {
        /// Index in the message's `content` array.
        index: u32,
        /// What changed.
        delta: Delta,
    },
    /// A content block is complete.
    ContentBlockStop {
        /// Index in the message's `content` array.
        index: u32,
    },
    /// Top-level message delta (final stop_reason, usage so far, etc.).
    MessageDelta {
        /// What changed about the top-level message.
        delta: MessageDelta,
        /// Cumulative token usage.
        usage: Usage,
    },
    /// Final event for a message.
    MessageStop,
    /// Heartbeat keep-alive; ignore.
    Ping,
    /// Provider-side error; usually means the connection will close.
    #[serde(rename = "error")]
    Error {
        /// Error payload as Anthropic sent it.
        error: Value,
    },
    /// Any event type we don't know about. Forward-compatible escape hatch.
    #[serde(other)]
    Unknown,
}

/// The message shell carried by [`Event::MessageStart`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Anthropic message id (`msg_…`).
    pub id: String,
    /// Always `"assistant"` in v1.
    pub role: String,
    /// Model used.
    #[serde(default)]
    pub model: Option<String>,
    /// Stop reason if known at start time (usually null).
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Token usage as known at start.
    #[serde(default)]
    pub usage: Option<Usage>,
}

/// One content block (passed by [`Event::ContentBlockStart`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Plain text block.
    Text {
        /// Initial text (often empty; deltas append).
        #[serde(default)]
        text: String,
    },
    /// Tool-use block.
    ToolUse {
        /// Tool-call id (`toolu_…`).
        id: String,
        /// Tool name.
        name: String,
        /// Tool args (often `{}` initially; populated by partial JSON deltas).
        #[serde(default)]
        input: Value,
    },
    /// Extended-thinking block.
    Thinking {
        /// Initial thinking text.
        #[serde(default)]
        thinking: String,
    },
    /// Server-tool-use block (web_search, code_execution, etc.).
    ServerToolUse {
        /// Server tool name.
        name: String,
        /// Args passed to the tool.
        #[serde(default)]
        input: Value,
    },
    /// Anything else.
    #[serde(other)]
    Other,
}

/// One delta extending an in-progress content block.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Delta {
    /// Text-block delta.
    TextDelta {
        /// Appended text fragment.
        text: String,
    },
    /// Tool-use input streaming as partial JSON.
    InputJsonDelta {
        /// Partial JSON fragment to append.
        partial_json: String,
    },
    /// Extended-thinking text delta.
    ThinkingDelta {
        /// Appended thinking text.
        thinking: String,
    },
    /// Signature delta for the thinking block.
    SignatureDelta {
        /// Cryptographic signature fragment.
        signature: String,
    },
    /// Anything else.
    #[serde(other)]
    Other,
}

/// Top-level message delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDelta {
    /// Final stop reason once known.
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Stop sequence that was hit, if any.
    #[serde(default)]
    pub stop_sequence: Option<String>,
}

/// Token usage. Field set evolves; we keep optional everywhere.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Plain input tokens.
    #[serde(default)]
    pub input_tokens: Option<u64>,
    /// Output tokens generated.
    #[serde(default)]
    pub output_tokens: Option<u64>,
    /// Tokens served from prompt cache.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    /// Tokens written to prompt cache.
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
}
