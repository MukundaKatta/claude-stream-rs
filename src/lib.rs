//! Parse Anthropic's Server-Sent Events stream into typed events.
//!
//! No SDK dependency. Build it into whatever HTTP client / async runtime
//! you already have: feed [`EventParser::feed`] bytes as they arrive,
//! drain ready events with [`EventParser::next_event`].
//!
//! # Quick start
//!
//! ```
//! use claude_stream::{EventParser, Event};
//!
//! // Wire bytes Anthropic sent us:
//! let chunk = b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"role\":\"assistant\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n";
//!
//! let mut parser = EventParser::new();
//! parser.feed(chunk);
//!
//! // Drain everything ready:
//! while let Some(event) = parser.next_event().unwrap() {
//!     match event {
//!         Event::MessageStart { message } => println!("started: {}", message.id),
//!         Event::ContentBlockDelta { delta, .. } => {
//!             if let claude_stream::Delta::TextDelta { text } = delta {
//!                 print!("{text}");
//!             }
//!         }
//!         _ => {}
//!     }
//! }
//! ```
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rust_2018_idioms)]

mod event;
mod parser;

pub use crate::event::{ContentBlock, Delta, Event, Message, MessageDelta, ParseError, Usage};
pub use crate::parser::EventParser;
