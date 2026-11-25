use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use milo_parser::{ERROR_NONE, Parser, STATE_ERROR};

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
