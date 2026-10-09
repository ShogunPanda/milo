#![no_main]
#[macro_use]
extern crate libfuzzer_sys;

use core::ffi::c_void;

use milo_fuzz::{MessageFraming, MAX_FEED};
use milo_parser::{CALLBACK_ACTIVE_ALL, Parser};

/// Layer 3: request-smuggling oracle.
///
/// Per message, we capture how Milo framed the body, then assert that it never
/// *accepted* a message carrying a framing ambiguity that RFC 9112 §6.3 treats as
/// a MUST-NOT:
///
///  * `Content-Length` together with `Transfer-Encoding` (CL.TE / TE.CL),
///  * a `Transfer-Encoding` whose last coding is not `chunked`,
///  * duplicate / conflicting `Content-Length` values.
///
/// Milo is expected to reject all of these, so a panic here is a real finding
/// (`cargo-fuzz` records and minimizes it). This is the self-contained
/// conformance tripwire. The differential oracle against a reference parser
/// (`llhttp`) is described in the `oracle` module and enabled with
/// `--features oracle`.

/// Per-message state gathered while parsing.
struct SmugContext {
  input: Vec<u8>,
  messages: Vec<MessageFraming>,
  current: MessageFraming,
  current_header_name: Vec<u8>,
}

impl SmugContext {
  fn new() -> Self {
    SmugContext {
      input: Vec::new(),
      messages: Vec::new(),
      current: MessageFraming::default(),
      current_header_name: Vec::new(),
    }
  }
}

/// Read a callback payload (`from`/`size`) from the input we passed to `parse`.
/// With `manage_unconsumed` disabled the offsets align exactly with `input`.
fn read_payload(p: &Parser, from: usize, size: usize) -> Vec<u8> {
  let ctx = unsafe { &*(p.context as *const SmugContext) };
  if size == 0 {
    return Vec::new();
  }
  ctx.input.get(from..from + size).unwrap_or_default().to_vec()
}

extern "C" fn on_message_start(p: &mut Parser, _from: usize, _size: usize) {
  let ctx = unsafe { &mut *(p.context as *mut SmugContext) };
  ctx.current = MessageFraming::default();
  ctx.current_header_name.clear();
}

extern "C" fn on_headers(p: &mut Parser, _from: usize, _size: usize) {
  let ctx = unsafe { &mut *(p.context as *mut SmugContext) };
  ctx.current.has_content_length = p.has_content_length;
  ctx.current.has_transfer_encoding = p.has_transfer_encoding;
  ctx.current.has_chunked_transfer_encoding = p.has_chunked_transfer_encoding;
}

extern "C" fn on_header_name(p: &mut Parser, from: usize, size: usize) {
  let ctx = unsafe { &mut *(p.context as *mut SmugContext) };
  ctx.current_header_name = read_payload(p, from, size);
}

extern "C" fn on_header_value(p: &mut Parser, from: usize, size: usize) {
  let ctx = unsafe { &mut *(p.context as *mut SmugContext) };
  let value = read_payload(p, from, size);
  let name = ctx.current_header_name.clone();

  if name.eq_ignore_ascii_case(b"content-length") {
    if let Ok(n) = parse_u64(&value) {
      ctx.current.content_lengths.push(n);
    }
  }
}

extern "C" fn on_message_complete(p: &mut Parser, _from: usize, _size: usize) {
  let ctx = unsafe { &mut *(p.context as *mut SmugContext) };
  ctx.current.consumed = p.position;
  ctx.messages.push(ctx.current.clone());
}

fn parse_u64(bytes: &[u8]) -> Result<u64, ()> {
  if bytes.is_empty() {
    return Err(());
  }
  let mut out: u64 = 0;
  for &b in bytes {
    if !b.is_ascii_digit() {
      return Err(());
    }
    out = out
      .checked_mul(10)
      .and_then(|o| o.checked_add((b - b'0') as u64))
      .ok_or(())?;
  }
  Ok(out)
}

/// Parse `data` once in the given mode and return the per-message framing.
fn parse_and_collect(data: &[u8], is_request: Option<bool>) -> Vec<MessageFraming> {
  let mut p = Parser::new();
  if let Some(req) = is_request {
    p.autodetect = false;
    p.is_request = req;
  }

  let ctx = Box::new(SmugContext::new());
  p.context = Box::into_raw(ctx) as *mut c_void;
  p.active_callbacks = CALLBACK_ACTIVE_ALL;
  p.callbacks.on_message_start = on_message_start;
  p.callbacks.on_headers = on_headers;
  p.callbacks.on_header_name = on_header_name;
  p.callbacks.on_header_value = on_header_value;
  p.callbacks.on_message_complete = on_message_complete;

  let limit = data.len().min(MAX_FEED);
  {
    let ctx = unsafe { &mut *(p.context as *mut SmugContext) };
    ctx.input = data.to_vec();
  }

  let _ = p.parse(data.as_ptr(), limit);

  // Recover the context Box so it is freed (no leak per fuzz iteration).
  let ctx = unsafe { Box::from_raw(p.context as *mut SmugContext) };
  ctx.messages
}

#[cfg(feature = "oracle")]
mod oracle {
  //! Differential oracle against a reference parser.
  //!
  //! Enable with `cargo fuzz run smuggling --features oracle`.
  //!
  //! Workflow:
  //!  1. Parse `data` with llhttp (`llhttp` optional dependency in Cargo.toml)
  //!     and read `on_headers_complete` to snapshot `F_CONTENT_LENGTH`,
  //!     `F_CHUNKED` and `F_TRANSFER_ENCODING`.
  //!  2. Compare the derived [`milo_fuzz::Framing`] and the end-of-message
  //!     offset against Milo's for the same bytes.
  //!  3. Any disagreement => `panic!`, which `cargo-fuzz` records and minimizes.
  //!
  //! Pin the `llhttp` crate to the same upstream version Milo imports its
  //! conformance suite from, then implement `fn compare(data, messages)`.
}

fuzz_target!(|data: &[u8]| {
  // autodetect mode, plus forced client and server modes.
  for is_request in [None, Some(true), Some(false)] {
    for m in parse_and_collect(data, is_request) {
      if m.is_smuggling_primitive() {
        panic!(
          "smuggling primitive accepted: consumed={} cl={} te={} chunked={} cls={:?}",
          m.consumed, m.has_content_length, m.has_transfer_encoding,
          m.has_chunked_transfer_encoding, m.content_lengths
        );
      }
    }
  }
});
