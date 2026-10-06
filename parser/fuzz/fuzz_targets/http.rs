#![no_main]
#[macro_use]
extern crate libfuzzer_sys;

use arbitrary::{Arbitrary, Unstructured};
use milo_fuzz::MAX_FEED;
use milo_parser::{CALLBACK_ACTIVE_ALL, Parser};

/// Layer 2: structure-aware generation. Builds plausible HTTP/1.1 messages so
/// the fuzzer reaches deep state-machine paths (chunked bodies, trailers, body
/// via Content-Length) that random bytes rarely construct. We still feed raw
/// bytes too, so mutations can *break* the structure — that is where smuggling
/// lives.
#[derive(Arbitrary, Debug)]
struct FuzzHeader {
  name: Vec<u8>,
  value: Vec<u8>,
}

#[derive(Arbitrary, Debug)]
struct FuzzMessage {
  is_request: bool,
  method: Vec<u8>,
  url: Vec<u8>,
  version: Vec<u8>,
  headers: Vec<FuzzHeader>,
  body: Vec<u8>,
  chunked: bool,
  trailer_headers: Vec<FuzzHeader>,
}

impl FuzzMessage {
  fn to_wire(&self) -> Vec<u8> {
    let mut out = Vec::with_capacity(256);

    if self.is_request {
      out.extend_from_slice(&self.method);
      out.extend_from_slice(b" ");
      out.extend_from_slice(&self.url);
      out.extend_from_slice(b" ");
      out.extend_from_slice(&self.version);
      out.extend_from_slice(b"\r\n");
    } else {
      out.extend_from_slice(b"HTTP/1.1 200 OK\r\n");
    }

    for h in &self.headers {
      out.extend_from_slice(&h.name);
      out.extend_from_slice(b": ");
      out.extend_from_slice(&h.value);
      out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");

    if self.chunked {
      let mut left = self.body.len();
      let mut i = 0;
      while left > 0 {
        let chunk = (left % 17 + 1).min(left);
        out.extend_from_slice(format!("{:x}\r\n", chunk).as_bytes());
        out.extend_from_slice(&self.body[i..i + chunk]);
        out.extend_from_slice(b"\r\n");
        i += chunk;
        left -= chunk;
      }
      out.extend_from_slice(b"0\r\n");
      for t in &self.trailer_headers {
        out.extend_from_slice(&t.name);
        out.extend_from_slice(b": ");
        out.extend_from_slice(&t.value);
        out.extend_from_slice(b"\r\n");
      }
      out.extend_from_slice(b"\r\n");
    } else {
      out.extend_from_slice(&self.body);
    }

    out
  }
}

fn run(bytes: &[u8]) {
  let limit = bytes.len().min(MAX_FEED);
  let mut p = Parser::new();
  p.active_events = CALLBACK_ACTIVE_ALL;
  let _ = p.parse(bytes.as_ptr(), limit);
}

fuzz_target!(|data: &[u8]| {
  if let Ok(msg) = FuzzMessage::arbitrary(&mut Unstructured::new(data)) {
    let wire = msg.to_wire();
    run(&wire);
  }
  // Always also feed the raw bytes so structure-valid mutations are explored.
  run(data);
});
