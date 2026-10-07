//! Conformance smoke tests for the request-smuggling invariants.
//!
//! These are plain `#[test]`s (not fuzz targets) so they can be run under Miri to
//! catch undefined behaviour in the parser's `unsafe` code, and used to pin a
//! regression once a fuzzing finding is fixed:
//!
//! ```sh
//! cargo +nightly miri test --test conformance
//! ```

use core::ffi::c_void;

use milo_parser::{CALLBACK_ACTIVE_ALL, ERROR_NONE, Parser, STATE_ERROR};

fn error_code(data: &[u8], is_request: bool) -> u8 {
  let mut p = Parser::new();
  p.autodetect = false;
  p.is_request = is_request;
  let _ = p.parse(data.as_ptr(), data.len());
  p.error_code
}

/// A message is "accepted" when it did not enter the error state.
fn accepted(data: &[u8], is_request: bool) -> bool {
  let mut p = Parser::new();
  p.autodetect = false;
  p.is_request = is_request;
  let _ = p.parse(data.as_ptr(), data.len());
  p.state != STATE_ERROR
}

// --- RFC 9112 §6.3 MUST-NOT framing ambiguities. Milo must reject these. ---

#[test]
fn rejects_content_length_and_transfer_encoding_together() {
  for input in [
    &b"POST / HTTP/1.1\r\nContent-Length: 6\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"[..],
    &b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\nContent-Length: 6\r\n\r\n"[..],
  ] {
    assert_ne!(error_code(input, true), ERROR_NONE, "CL+TE must be rejected, got accepted: {input:?}");
  }
}

#[test]
fn rejects_duplicate_content_length() {
  let input = b"POST / HTTP/1.1\r\nContent-Length: 4\r\nContent-Length: 6\r\n\r\n";
  assert_ne!(error_code(input, true), ERROR_NONE, "duplicate CL must be rejected");
}

#[test]
fn rejects_transfer_encoding_not_ending_in_chunked() {
  for input in [
    &b"POST / HTTP/1.1\r\nTransfer-Encoding: gzip\r\n\r\n"[..],
    &b"POST / HTTP/1.1\r\nTransfer-Encoding: gzip, chunked, br\r\n\r\n"[..],
  ] {
    assert_ne!(error_code(input, true), ERROR_NONE, "TE not ending in chunked must be rejected: {input:?}");
  }
}

// --- Well-formed messages must still be accepted. ---

#[test]
fn accepts_valid_requests() {
  for input in [
    &b"GET / HTTP/1.1\r\nHost: x\r\n\r\n"[..],
    &b"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\nabcd"[..],
  ] {
    assert!(accepted(input, true), "valid request must be accepted: {input:?}");
  }
}

// --- Callback-driven framing collection (the mechanism the smuggling target
//     relies on). Verifies on_headers / on_header_name / on_header_value /
//     on_message_complete fire and snapshot the expected framing. ---

#[derive(Default)]
struct Ctx {
  input: Vec<u8>,
  completed: Vec<(bool, bool, bool, Vec<u64>)>, // (has_cl, has_te, has_chunked, cls)
  cur: (bool, bool, bool, Vec<u64>),
  name: Vec<u8>,
}

fn on_headers(p: &mut Parser, _f: usize, _s: usize) {
  let c = unsafe { &mut *(p.context as *mut Ctx) };
  c.cur = (p.has_content_length, p.has_transfer_encoding, p.has_chunked_transfer_encoding, c.cur.3.clone());
}

fn on_header_name(p: &mut Parser, from: usize, size: usize) {
  let c = unsafe { &mut *(p.context as *mut Ctx) };
  c.name = c.input.get(from..from + size).unwrap_or_default().to_vec();
}

fn on_header_value(p: &mut Parser, from: usize, size: usize) {
  let c = unsafe { &mut *(p.context as *mut Ctx) };
  let value = c.input.get(from..from + size).unwrap_or_default();
  if c.name.eq_ignore_ascii_case(b"content-length") {
    if let Some(n) = value.iter().try_fold(0u64, |acc, b| acc.checked_mul(10).and_then(|o| o.checked_add((b - b'0') as u64))) {
      c.cur.3.push(n);
    }
  }
}

fn on_message_complete(p: &mut Parser, _f: usize, _s: usize) {
  let c = unsafe { &mut *(p.context as *mut Ctx) };
  c.completed.push(c.cur.clone());
  c.cur = Default::default();
}

#[test]
fn callback_framing_collection() {
  let input = b"POST / HTTP/1.1\r\nContent-Length: 4\r\n\r\nabcd";
  let mut p = Parser::new();
  let ctx = Box::new(Ctx { input: input.to_vec(), ..Default::default() });
  p.context = Box::into_raw(ctx) as *mut c_void;
  p.active_callbacks = CALLBACK_ACTIVE_ALL;
  p.callbacks.on_headers = on_headers;
  p.callbacks.on_header_name = on_header_name;
  p.callbacks.on_header_value = on_header_value;
  p.callbacks.on_message_complete = on_message_complete;
  let _ = p.parse(input.as_ptr(), input.len());

  let ctx = unsafe { Box::from_raw(p.context as *mut Ctx) };
  assert_eq!(ctx.completed.len(), 1, "one completed message expected");
  let (has_cl, has_te, has_chunked, cls) = ctx.completed[0].clone();
  assert!(has_cl, "on_headers must snapshot has_content_length");
  assert!(!has_te);
  assert!(!has_chunked);
  assert_eq!(cls, vec![4], "on_header_value must collect the Content-Length value");
}
