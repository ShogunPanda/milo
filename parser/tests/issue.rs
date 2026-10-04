mod helpers;

use milo_parser::{ERROR_NONE, Parser, STATE_ERROR};

use crate::helpers::{create_parser, parse};

fn response_parser() -> Parser {
  let mut parser = create_parser();
  parser.autodetect = false;
  parser.is_request = false;
  parser
}

fn field_messages(value: &[u8]) -> [Vec<u8>; 3] {
  let fields: [(&[u8], &[u8]); 3] = [
    (b"HTTP/1.1 200 OK\r\nX-Long: ", b"\r\nContent-Length: 0\r\n\r\n"),
    (
      b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-Long: ",
      b"\r\n\r\n",
    ),
    (b"HTTP/1.1 200 ", b"\r\nContent-Length: 0\r\n\r\n"),
  ];
  fields.map(|(prefix, suffix)| [prefix, value, suffix].concat())
}

// Cover SIMD block boundaries and scalar tails in both field scanners.
#[test]
#[allow(non_snake_case)]
fn issue_22__field_values_reject_controls() {
  let mut parser = response_parser();
  parser.active_callbacks = 0;

  for byte in (0u8..0x20).chain([0x7f]).filter(|byte| *byte != b'\t') {
    for offset in [0, 7, 8, 15, 16, 20, 31, 32] {
      for trailing in [0, 20] {
        let mut value = vec![b'a'; offset];
        value.push(byte);
        value.extend(vec![b'a'; trailing]);
        for message in field_messages(&value) {
          parser.reset(false);
          parser.parse(message.as_ptr(), message.len());
          assert_eq!(parser.state, STATE_ERROR, "Accepted invalid field: {message:?}");
        }
      }
    }
  }
}

// HTAB and every obs-text byte remain valid, including across SIMD boundaries.
#[test]
#[allow(non_snake_case)]
fn issue_22__field_values_allow_tab_and_obs_text() {
  let mut parser = response_parser();
  // Raw obs-text is not necessarily UTF-8, so bypass the text-decoding callbacks.
  parser.active_callbacks = 0;

  for byte in [b'\t'].into_iter().chain(0x80..=0xff) {
    for offset in [0, 7, 8, 15, 16, 20, 31, 32] {
      for trailing in [0, 20] {
        let mut value = vec![b'a'; offset];
        value.push(byte);
        value.extend(vec![b'a'; trailing]);
        for message in field_messages(&value) {
          parser.reset(false);
          parser.parse(message.as_ptr(), message.len());
          assert_ne!(parser.state, STATE_ERROR, "Rejected valid field: {message:?}");
          assert_eq!(parser.error_code, ERROR_NONE);
        }
      }
    }
  }
}

// Bare LF is rejected in HTTP framing.
#[test]
#[allow(non_snake_case)]
fn issue_22__bare_lf_rejected() {
  let mut parser = response_parser();
  let message = "HTTP/1.1 200 OK\r\nHeader: value\nContent-Length: 0\r\n\r\n";

  parse(&mut parser, message);

  assert_eq!(parser.state, STATE_ERROR);
}
