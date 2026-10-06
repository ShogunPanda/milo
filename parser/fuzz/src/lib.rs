//! Shared helpers for the Milo cargo-fuzz targets.
//!
//! This crate is **not** part of the normal Milo build. It is only compiled
//! when `cargo-fuzz` builds a target in `parser/fuzz`. See `../README.md`.

use milo_parser::Parser;

/// How a parser chose to frame the body of the current message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framing {
  /// No `Content-Length` and no `Transfer-Encoding`: the body runs until the
  /// connection closes.
  Closure,
  /// Body is bounded by a single `Content-Length`.
  ContentLength(u64),
  /// `Transfer-Encoding` whose last coding is `chunked`.
  Chunked,
  /// `Transfer-Encoding` present but the last coding is **not** `chunked`.
  /// This is a protocol violation and, when accepted, a smuggling primitive.
  UnchunkedTransfer,
}

/// Derive the body framing from the parser's public post-parse flags.
///
/// Call this while the parser still reflects the message of interest (for
/// example from an `on_headers` callback, or immediately after a `parse` call
/// for a single-message buffer).
pub fn framing(parser: &Parser) -> Framing {
  if parser.has_transfer_encoding {
    if parser.has_chunked_transfer_encoding {
      Framing::Chunked
    } else {
      Framing::UnchunkedTransfer
    }
  } else if parser.has_content_length {
    Framing::ContentLength(parser.content_length)
  } else {
    Framing::Closure
  }
}

/// A single parsed message's framing, captured via callbacks while parsing.
#[derive(Debug, Clone, Default)]
pub struct MessageFraming {
  /// `parser.position` at the moment the message completed.
  pub consumed: usize,
  pub has_content_length: bool,
  pub has_transfer_encoding: bool,
  pub has_chunked_transfer_encoding: bool,
  /// Every numeric value seen for `Content-Length` in this message.
  pub content_lengths: Vec<u64>,
}

impl MessageFraming {
  /// True when Milo accepted a message that encodes an RFC 9112 §6.3 framing
  /// ambiguity (the classic smuggling primitives).
  ///
  /// Milo is expected to reject all of these, so a positive result is a
  /// regression / bug. This is the self-contained "conformance tripwire".
  pub fn is_smuggling_primitive(&self) -> bool {
    // Content-Length and Transfer-Encoding together.
    if self.has_transfer_encoding && self.has_content_length {
      return true;
    }
    // Transfer-Encoding present but not terminated by chunked.
    if self.has_transfer_encoding && !self.has_chunked_transfer_encoding {
      return true;
    }
    // Duplicate / conflicting Content-Length values.
    if self.content_lengths.len() > 1 {
      let first = self.content_lengths[0];
      if self.content_lengths.iter().any(|&v| v != first) {
        return true;
      }
    }
    false
  }
}

/// Maximum number of bytes we feed to the parser per `parse` call.
///
/// The parser's event buffer is a fixed 64 KiB; feeding more just exercises the
/// suspend/retain paths and wastes fuzzing budget.
pub const MAX_FEED: usize = 64 * 1024;
