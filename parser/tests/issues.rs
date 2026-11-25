mod helpers;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use milo_parser::{
  ERROR_NONE, EVENT_ACTIVE_ON_HEADERS, EVENT_HEADERS, METHOD_CONNECT, METHOD_POST, Parser, STATE_ERROR, STATE_START,
  STATE_TUNNEL,
};

use crate::helpers::{create_parser, parse};

struct TrackingAllocator;

static WATCHED_POINTER: AtomicUsize = AtomicUsize::new(0);
static FREED_SIZE: AtomicUsize = AtomicUsize::new(0);

thread_local! {
  // Count only this test thread's allocations, excluding test-runner activity.
  static TRACK_MEMORY: Cell<bool> = const { Cell::new(false) };
  static LIVE_BYTES: Cell<isize> = const { Cell::new(0) };
}

// Keep the allocator in this integration-test binary so other suites cannot
// interfere with the retained-buffer lifetime checks.
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator;

// SAFETY: Every allocation and deallocation is forwarded unchanged to System.
unsafe impl GlobalAlloc for TrackingAllocator {
  unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    // SAFETY: The caller supplies the valid allocation layout.
    let pointer = unsafe { System.alloc(layout) };
    if !pointer.is_null() && TRACK_MEMORY.try_with(Cell::get).unwrap_or(false) {
      let _ = LIVE_BYTES.try_with(|bytes| bytes.set(bytes.get() + layout.size() as isize));
    }
    pointer
  }

  unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
    if TRACK_MEMORY.try_with(Cell::get).unwrap_or(false) {
      let _ = LIVE_BYTES.try_with(|bytes| bytes.set(bytes.get() - layout.size() as isize));
    }
    if WATCHED_POINTER
      .compare_exchange(pointer as usize, 0, Ordering::SeqCst, Ordering::SeqCst)
      .is_ok()
    {
      FREED_SIZE.store(layout.size(), Ordering::SeqCst);
    }
    // SAFETY: The pointer and layout are forwarded from the allocator caller.
    unsafe { System.dealloc(pointer, layout) }
  }
}

#[test]
#[allow(non_snake_case)]
fn issue_16__repeated_input_has_bounded_memory() {
  for input in [
    b"".as_slice(),
    b"GET / HTTP/1.1\r\nX-Test: ",
    b"GET / HTTP/1.1\r\nX-Test: \0",
    // Declared body lengths must not cause proportional allocations.
    b"POST / HTTP/1.1\r\nContent-Length: 9999999999999999999\r\n\r\n",
    b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\nffffffffffffffff\r\n",
  ] {
    for managed in [false, true] {
      TRACK_MEMORY.set(true);
      let baseline = LIVE_BYTES.get();
      for _ in 0..1024 {
        let mut parser = Parser::new();
        parser.manage_unconsumed = managed;
        parser.parse(input.as_ref());

        // Empty retries must not leak copies of a pending line or repeat errors.
        let retained_len = parser.unconsumed().len();
        let parsed = parser.parsed;
        let state = parser.state;
        let live = LIVE_BYTES.get();
        for _ in 0..16 {
          assert_eq!(parser.parse(b""), 0);
          assert_eq!(parser.unconsumed().len(), retained_len);
          assert_eq!(parser.parsed, parsed);
          assert_eq!(parser.state, state);
          assert_eq!(LIVE_BYTES.get(), live);
        }

        parser.finish();
        parser.reset(false);
        assert_eq!(parser.error_code, ERROR_NONE);
        assert!(parser.unconsumed().is_empty());
        parser.parse(input.as_ref());
        drop(parser);
        assert_eq!(LIVE_BYTES.get(), baseline, "input={input:?}, managed={managed}");
      }
      TRACK_MEMORY.set(false);
    }
  }
}

#[test]
#[allow(non_snake_case)]
fn issue_16__error_retries_do_not_accumulate_input() {
  for managed in [false, true] {
    let mut parser = Parser::new();
    parser.manage_unconsumed = managed;
    let invalid = b"GET / HTTP/1.1\r\nX-Test: \0";
    parser.parse(invalid.as_ref());
    assert_eq!(parser.state, STATE_ERROR);
    let code = parser.error_code;
    let description = parser.error_description_str().to_string();
    let retained = parser.unconsumed().as_ptr();
    let retained_len = parser.unconsumed().len();
    let parsed = parser.parsed;

    TRACK_MEMORY.set(true);
    let baseline = LIVE_BYTES.get();
    for _ in 0..1024 {
      assert_eq!(parser.parse(invalid.as_ref()), 0);
      assert_eq!(parser.state, STATE_ERROR);
      assert_eq!(parser.error_code, code);
      assert_eq!(parser.error_description_str(), description);
      assert_eq!(parser.unconsumed().len(), retained_len);
      assert_eq!(parser.unconsumed().as_ptr(), retained);
      assert_eq!(parser.parsed, parsed);
      assert_eq!(parser.position, 0);
      assert_eq!(LIVE_BYTES.get(), baseline);
    }
    TRACK_MEMORY.set(false);

    parser.reset(false);
    let valid = b"GET / HTTP/1.1\r\n\r\n";
    assert_eq!(parser.parse(valid.as_ref()), valid.len());
    assert_eq!(parser.error_code, ERROR_NONE);
  }
}

#[test]
#[allow(non_snake_case)]
fn issue_16__release_retained_input() {
  let mut parser = Parser::new();
  parser.manage_unconsumed = true;
  let prefix = b"GET / HTTP/1.1\r\nX-Test: ";
  assert_eq!(parser.parse(prefix.as_ref()), 16);
  assert_eq!(parser.unconsumed().len(), 8);

  // Empty input must keep the existing allocation and pending bytes.
  for input in [b"a".as_slice(), b"", b"b"] {
    let previous_len = parser.unconsumed().len();
    let pointer = parser.unconsumed().as_ptr();
    assert_eq!(parser.parse(input), 0);
    assert_eq!(parser.unconsumed().len(), previous_len + input.len());
    if input.is_empty() {
      assert_eq!(parser.unconsumed().as_ptr(), pointer);
    }
  }

  let previous_len = parser.unconsumed().len();
  WATCHED_POINTER.store(parser.unconsumed().as_ptr() as usize, Ordering::SeqCst);
  let suffix = b"\r\n\r\n";
  assert_eq!(parser.parse(suffix.as_ref()), previous_len + suffix.len());
  assert_eq!(parser.parse(b""), 0);
  assert_eq!(WATCHED_POINTER.load(Ordering::SeqCst), 0);
  assert!(FREED_SIZE.load(Ordering::SeqCst) >= previous_len);
  assert!(parser.unconsumed().is_empty());
  assert_eq!(parser.unconsumed().len(), 0);
  assert_eq!(parser.error_code, ERROR_NONE);

  // Reset and destruction must also release pending input, including after
  // malformed input has moved the parser into its terminal error state.
  for input in [prefix.as_slice(), b"GET / HTTP/1.1\r\nX-Test: \0"] {
    for reset in [false, true] {
      parser.reset(false);
      parser.parse(input.as_ref());
      assert!(!parser.unconsumed().is_empty());
      if input.ends_with(b"\0") {
        assert_eq!(parser.state, STATE_ERROR);
      }
      let previous_len = parser.unconsumed().len();
      WATCHED_POINTER.store(parser.unconsumed().as_ptr() as usize, Ordering::SeqCst);
      // Disabling retention must not lose ownership of an existing buffer.
      parser.manage_unconsumed = false;
      if reset {
        parser.reset(false);
        assert!(parser.unconsumed().is_empty());
        assert_eq!(parser.unconsumed().len(), 0);
      } else {
        drop(parser);
        parser = Parser::new();
      }
      assert_eq!(WATCHED_POINTER.load(Ordering::SeqCst), 0);
      assert!(FREED_SIZE.load(Ordering::SeqCst) >= previous_len);
      parser.manage_unconsumed = true;
    }
  }
}

#[test]
fn managed_input_waits_for_split_header_ending() {
  let mut parser = Parser::new();
  parser.manage_unconsumed = true;

  assert_eq!(parser.parse(b"GET / HTTP/1.1\r\nX-Test: first"), 16);
  assert_eq!(parser.parse(b" second"), 0);
  assert_eq!(parser.parse(b"\r"), 0);
  assert_eq!(parser.parse(b""), 0);
  assert_eq!(parser.parse(b"\n\r\n"), b"X-Test: first second\r\n\r\n".len());
  assert_eq!(parser.error_code, ERROR_NONE);
  assert!(parser.unconsumed().is_empty());
}

#[test]
fn managed_input_reuses_only_the_first_line_ending() {
  let mut parser = Parser::new();
  parser.manage_unconsumed = true;
  let prefix = b"GET /account HTTP/1.";
  let suffix = b"1\r\nX-Test: value\r\n\r\n";

  assert_eq!(parser.parse(prefix), 0);
  assert_eq!(parser.parse(suffix), prefix.len() + suffix.len());
  assert_eq!(parser.error_code, ERROR_NONE);
  assert!(parser.unconsumed().is_empty());
}

#[test]
fn parse_callbacks_can_read_the_current_input() {
  type Observations = Vec<(Vec<u8>, Vec<u8>)>;

  extern "C" fn on_header_value(parser: &mut Parser, at: usize, len: usize) {
    // SAFETY: This callback reads the current input before returning.
    let input = unsafe { parser.current_input() };
    if input.is_empty() {
      return;
    }
    let observation = (input.to_vec(), input[at..at + len].to_vec());
    // SAFETY: The test keeps the observations alive and lends them to this parser.
    let observations = unsafe { &mut *parser.context.cast::<Observations>() };
    observations.push(observation);
    parser.manage_unconsumed = false;
  }

  let mut observations = Observations::new();
  let mut parser = Parser::new();
  parser.context = core::ptr::from_mut(&mut observations).cast();
  parser.active_callbacks = milo_parser::CALLBACK_ACTIVE_ON_HEADER_VALUE;
  parser.callbacks.on_header_value = on_header_value;

  let request = b"GET / HTTP/1.1\r\nX-Test: value\r\n\r\n";
  assert_eq!(parser.parse(request), request.len());
  assert!(observations.is_empty());

  parser.reset(false);
  parser.manage_unconsumed = true;
  assert_eq!(parser.parse(b"GET / HTTP/1.1\r\nX-Test: va"), 16);
  assert_eq!(parser.parse(b"lue\r\nX-Next: sta"), 15);
  assert_eq!(parser.position, 15);
  assert_eq!(parser.parsed, 31);
  assert_eq!(parser.unconsumed(), b"X-Next: sta");
  // SAFETY: The parser owns the last aggregate until the next parse.
  assert_eq!(unsafe { parser.current_input() }, b"X-Test: value\r\nX-Next: sta");
  assert!(!parser.manage_unconsumed);
  assert_eq!(
    observations.pop(),
    Some((b"X-Test: value\r\nX-Next: sta".to_vec(), b"value".to_vec()))
  );

  parser.manage_unconsumed = true;
  assert_eq!(parser.parse(b"\r\n\r\n"), 15);
  assert_eq!(parser.parsed, 46);
  assert!(parser.unconsumed().is_empty());
  assert_eq!(observations.pop(), Some((b"X-Next: sta\r\n\r\n".to_vec(), b"sta".to_vec())));
  // SAFETY: Even without a retained suffix, the aggregate still backs the batch.
  assert_eq!(unsafe { parser.current_input() }, b"X-Next: sta\r\n\r\n");
  assert!(!parser.manage_unconsumed);
  assert_eq!(parser.parse(request), request.len());
  assert!(observations.is_empty());
  // SAFETY: The unmanaged call invalidated the prior batch.
  assert!(unsafe { parser.current_input() }.is_empty());
  parser.reset(false);
  // SAFETY: Reset invalidates the view.
  assert!(unsafe { parser.current_input() }.is_empty());
}

#[test]
fn managed_input_validates_lf_after_a_retained_cr() {
  let mut parser = Parser::new();
  parser.manage_unconsumed = true;

  assert_eq!(parser.parse(b"GET / HTTP/1.1\r\nX-Test: value\r"), 16);
  assert_eq!(parser.parse(b"x"), 0);
  assert_eq!(parser.error_code, milo_parser::ERROR_UNEXPECTED_CHARACTER);
  assert_eq!(parser.error_description_str(), "Expected CRLF");
}

#[test]
fn managed_input_checks_length_while_waiting() {
  let mut parser = Parser::new();
  parser.manage_unconsumed = true;
  parser.max_field_length = 12;

  assert_eq!(parser.parse(b"GET / HTTP/1.1\r\nX-Test: one"), 16);
  assert_eq!(parser.parse(b" "), 0);
  assert_eq!(parser.error_code, milo_parser::ERROR_UNEXPECTED_CHARACTER);
  assert_eq!(parser.error_description_str(), "Header line too long");
}

#[test]
fn managed_input_does_not_skip_lines_when_events_fill() {
  let request = b"GET / HTTP/1.1\r\nX-Test: value\r\n\r\n";
  let mut parser = Parser::new();
  parser.manage_unconsumed = true;
  parser.active_events = milo_parser::EVENT_ACTIVE_ON_HEADER_NAME;
  let input = request.repeat(parser.events().len() / 9 + 1);

  let first = parser.parse(&input);
  assert!(first > 0 && first < input.len());
  assert_eq!(parser.parse(b""), input.len() - first);
  assert_eq!(parser.error_code, ERROR_NONE);
  assert!(parser.unconsumed().is_empty());
}

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

#[test]
#[allow(non_snake_case)]
fn issue_23__field_value_ows() {
  use milo_parser::{
    ERROR_INVALID_CONTENT_LENGTH, ERROR_UNEXPECTED_CHARACTER, EVENT_ACTIVE_ON_DATA, EVENT_ACTIVE_ON_HEADER_VALUE,
    EVENT_ACTIVE_ON_MESSAGE_COMPLETE, EVENT_ACTIVE_ON_TRAILER_VALUE, EVENT_DATA, EVENT_END, EVENT_HEADER_VALUE,
    EVENT_MESSAGE_COMPLETE, EVENT_TRAILER_VALUE,
  };

  let whitespace = ["", " ", "\t", "  ", " \t", "\t ", "\t\t", " \t \t "];
  for leading in whitespace {
    for trailing in whitespace {
      for chunked in [false, true] {
        for events_enabled in [false, true] {
          let mut fields = if chunked {
            vec![
              ("Transfer-Encoding", "chunked"),
              ("Connection", "upgrade"),
              ("Upgrade", "h2c"),
              ("Trailer", "X-End"),
            ]
          } else {
            vec![("Content-Length", "2")]
          };
          fields.extend([("X-Test", "a \tb"), ("Custom", "value"), ("X-Empty", "")]);
          let mut message = String::from("HTTP/1.1 200 OK\r\n");
          for (name, value) in &fields {
            message.push_str(&format!("{name}:{leading}{value}{trailing}\r\n"));
          }
          message.push_str("\r\n");
          if chunked {
            message.push_str(&format!(
              "2\r\nok\r\n0\r\nX-End:{leading}done{trailing}\r\nX-Empty:{leading}{trailing}\r\n\r\n"
            ));
          } else {
            message.push_str("ok");
          }
          let mut parser = Parser::new();
          parser.autodetect = false;
          if events_enabled {
            parser.active_events = EVENT_ACTIVE_ON_HEADER_VALUE
              | EVENT_ACTIVE_ON_TRAILER_VALUE
              | EVENT_ACTIVE_ON_DATA
              | EVENT_ACTIVE_ON_MESSAGE_COMPLETE;
          }
          assert_eq!(
            parser.parse(message.as_ref()),
            message.len(),
            "{message:?}"
          );
          assert_eq!(parser.error_code, ERROR_NONE, "{message:?}");
          if events_enabled {
            let events = parser.events();
            let mut received = Vec::new();
            let mut cursor = 0;
            while events[cursor] != EVENT_END {
              let event = events[cursor];
              assert!(matches!(
                event,
                EVENT_HEADER_VALUE | EVENT_TRAILER_VALUE | EVENT_DATA | EVENT_MESSAGE_COMPLETE
              ));
              let at = u32::from_le_bytes(events[cursor + 1..cursor + 5].try_into().unwrap()) as usize;
              let len = u32::from_le_bytes(events[cursor + 5..cursor + 9].try_into().unwrap()) as usize;
              received.push((event, &message[at..at + len]));
              cursor += 9;
            }
            let mut expected: Vec<_> = fields.iter().map(|(_, value)| (EVENT_HEADER_VALUE, *value)).collect();
            expected.push((EVENT_DATA, "ok"));
            if chunked {
              expected.extend([(EVENT_TRAILER_VALUE, "done"), (EVENT_TRAILER_VALUE, "")]);
            }
            expected.push((EVENT_MESSAGE_COMPLETE, ""));
            assert_eq!(received, expected, "{message:?}");
          }
        }
      }
    }
  }

  for value in ["", " ", "\t", " \t ", "2 2", "2\t2"] {
    let mut parser = Parser::new();
    let message = format!("HTTP/1.1 200 OK\r\nContent-Length:{value}\r\n\r\n");
    parser.parse(message.as_ref());
    let expected = if value.contains('2') {
      ERROR_INVALID_CONTENT_LENGTH
    } else {
      ERROR_UNEXPECTED_CHARACTER
    };
    assert_eq!(parser.error_code, expected, "{message:?}");
  }
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
          parser.parse(message.as_ref());
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
          parser.parse(message.as_ref());
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
        parser.parse(message.as_ref()),
        message.len(),
        "{message}"
      );
      assert_eq!(parser.error_code, ERROR_NONE, "{message}");
      let events = parser.events();
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
      parser.parse(&[]);
      assert_eq!(parser.error_code, ERROR_NONE, "{message}");
      assert_eq!(parser.state == STATE_TUNNEL, connect || expected, "{message}");
    }
  }
}

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
          parser.max_fields_count = usize::MAX;
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
              let consumed = parser.parse(remaining.as_ref());
              let events = parser.events();
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
          assert_eq!(parser.parse(suffix.as_ref()), 0);
          assert_eq!(parser.state, STATE_ERROR);
        }
      }
    }
  }
}

#[test]
#[allow(non_snake_case)]
fn issue_63__no_content_with_content_length() {
  for length in [0, 10] {
    let mut parser = response_parser();
    let message = format!("HTTP/1.1 204 No Content\r\nContent-Length: {length}\r\n\r\n");

    assert_eq!(parse(&mut parser, &message), message.len());
    assert_eq!(parser.error_code, ERROR_NONE, "{}", parser.error_description_str());
    assert_eq!(parser.state, STATE_START);
    assert!(parser.has_content_length);
    assert_eq!(parser.content_length, length);
  }
}

#[test]
#[allow(non_snake_case)]
fn issue_63__no_body_status_with_transfer_encoding() {
  for status in ["204 No Content", "304 Not Modified"] {
    for encoding in ["chunked", "gzip"] {
      let mut parser = response_parser();
      let message = format!("HTTP/1.1 {status}\r\nTransfer-Encoding: {encoding}\r\n\r\n");

      assert_eq!(parse(&mut parser, &message), message.len());
      assert_eq!(parser.error_code, ERROR_NONE, "{}", parser.error_description_str());
      assert_eq!(parser.state, STATE_START);
      assert!(parser.has_transfer_encoding);
      assert_eq!(parser.has_chunked_transfer_encoding, encoding == "chunked");
    }
  }
}
