mod helpers;

#[test]
#[allow(non_snake_case)]
fn issue_26__preserve_events_before_error() {
  use milo_parser::{
    ERROR_UNSUPPORTED_HTTP_VERSION, EVENT_ACTIVE_ON_ERROR, EVENT_ACTIVE_ON_HEADER_VALUE, EVENT_ACTIVE_ON_HEADERS,
    EVENT_ACTIVE_ON_MESSAGE_COMPLETE, EVENT_END, EVENT_ERROR, EVENT_HEADER_VALUE, EVENT_MESSAGE_COMPLETE,
  };

  let suffix = "HTTP/9.9 garbage\r\n\r\n";
  for chunked in [false, true] {
    for split in [false, true] {
      for errors in [false, true] {
        // The largest case must suspend before completion and resume in a fresh batch.
        for padding in [0, 7276, 7277] {
          let body = if chunked {
            "Transfer-Encoding: chunked\r\n\r\n2\r\nok\r\n0\r\n\r\n"
          } else {
            "Content-Length: 2\r\n\r\nok"
          };
          let response = format!("HTTP/1.1 200 OK\r\n{}{body}", "X: a\r\n".repeat(padding));
          let inputs = if split {
            vec![response, suffix.into()]
          } else {
            vec![response + suffix]
          };
          let mut parser = Parser::new();
          parser.autodetect = false;
          parser.active_events =
            EVENT_ACTIVE_ON_HEADERS | EVENT_ACTIVE_ON_MESSAGE_COMPLETE | EVENT_ACTIVE_ON_HEADER_VALUE;
          if errors {
            parser.active_events |= EVENT_ACTIVE_ON_ERROR;
          }
          let mut received = Vec::new();
          for input in inputs {
            let mut remaining = input.as_bytes();
            while !remaining.is_empty() && parser.state != STATE_ERROR {
              let consumed = parser.parse(remaining.as_ptr(), remaining.len());
              // SAFETY: The live parser owns a 65536-byte event buffer.
              let events = unsafe { std::slice::from_raw_parts(parser.events, 65536) };
              let mut cursor = 0;
              while events[cursor] != EVENT_END {
                let event = events[cursor];
                received.push(event);
                cursor += match event {
                  EVENT_HEADERS => 19,
                  EVENT_ERROR => {
                    assert_eq!(events[cursor + 5], ERROR_UNSUPPORTED_HTTP_VERSION);
                    6
                  }
                  EVENT_MESSAGE_COMPLETE | EVENT_HEADER_VALUE => 9,
                  _ => panic!("Unexpected event: {event}"),
                };
                assert!(cursor < events.len());
              }
              assert!(consumed > 0 || parser.state == STATE_ERROR);
              remaining = &remaining[consumed..];
            }
          }
          let mut expected = vec![EVENT_HEADER_VALUE; padding + 1];
          expected.extend([EVENT_HEADERS, EVENT_MESSAGE_COMPLETE]);
          if errors {
            expected.push(EVENT_ERROR);
          }
          assert_eq!(
            received, expected,
            "chunked={chunked}, split={split}, errors={errors}, padding={padding}"
          );
          assert_eq!(parser.error_code, ERROR_UNSUPPORTED_HTTP_VERSION);
          assert_eq!(parser.state, STATE_ERROR);
          assert_eq!(parser.parse(suffix.as_ptr(), suffix.len()), 0);
          assert_eq!(parser.state, STATE_ERROR);
        }
      }
    }
  }
}

use milo_parser::{
  ERROR_NONE, EVENT_ACTIVE_ON_HEADERS, EVENT_HEADERS, METHOD_CONNECT, METHOD_POST, Parser, STATE_ERROR, STATE_TUNNEL,
};

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

#[test]
#[allow(non_snake_case)]
fn issue_25__headers_upgrade_metadata() {
  let mut cases: Vec<_> = [100, 101, 103, 200, 204, 301, 304, 400, 426, 500]
    .into_iter()
    .map(|status| (format!("HTTP/1.1 {status} Test"), status, false, false))
    .collect();
  cases.extend([
    ("POST / HTTP/1.1".into(), METHOD_POST as u16, true, false),
    (
      "CONNECT example.com:443 HTTP/1.1".into(),
      METHOD_CONNECT as u16,
      true,
      true,
    ),
    ("HTTP/1.1 200 Connection Established".into(), 200, false, true),
  ]);

  for (start, method_or_status, request, connect) in cases {
    for upgrade in [false, true] {
      let expected = upgrade && (request || method_or_status == 101);
      let headers = if upgrade {
        "Connection: upgrade\r\nUpgrade: h2c\r\n"
      } else {
        ""
      };
      let message = format!("{start}\r\n{headers}\r\n");
      let mut parser = Parser::new();
      parser.autodetect = false;
      parser.is_request = request;
      parser.suspend_after_headers = true;
      parser.active_events = EVENT_ACTIVE_ON_HEADERS;

      assert_eq!(
        parser.parse(message.as_ptr(), message.len()),
        message.len(),
        "{message}"
      );
      assert_eq!(parser.error_code, ERROR_NONE, "{message}");
      // SAFETY: The live parser owns the event buffer, and the complete headers emit
      // a 19-byte event.
      let events = unsafe { std::slice::from_raw_parts(parser.events, 19) };
      assert_eq!(events[0], EVENT_HEADERS, "{message}");
      assert_eq!(
        u16::from_le_bytes([events[5], events[6]]),
        method_or_status,
        "{message}"
      );
      assert_eq!(events[8], expected as u8, "{message}");

      // Supply CONNECT response context after parsing headers, before deciding the
      // body framing.
      if !request && connect {
        parser.is_connect = true;
      }
      parser.suspend_after_headers = false;
      parser.parse(message.as_ptr(), 0);
      assert_eq!(parser.error_code, ERROR_NONE, "{message}");
      assert_eq!(parser.state == STATE_TUNNEL, connect || expected, "{message}");
    }
  }
}
