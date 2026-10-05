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
        parser.parse(input.as_ptr(), input.len());

        // Empty retries must not leak copies of a pending line or repeat errors.
        let retained_len = parser.unconsumed_len;
        let parsed = parser.parsed;
        let state = parser.state;
        let live = LIVE_BYTES.get();
        for _ in 0..16 {
          assert_eq!(parser.parse(b"".as_ptr(), 0), 0);
          assert_eq!(parser.unconsumed_len, retained_len);
          assert_eq!(parser.parsed, parsed);
          assert_eq!(parser.state, state);
          assert_eq!(LIVE_BYTES.get(), live);
        }

        parser.finish();
        parser.reset(false);
        assert_eq!(parser.error_code, ERROR_NONE);
        assert!(parser.unconsumed.is_null());
        parser.parse(input.as_ptr(), input.len());
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
    parser.parse(invalid.as_ptr(), invalid.len());
    assert_eq!(parser.state, STATE_ERROR);
    let code = parser.error_code;
    let description = parser.error_description_str().to_string();
    let retained = parser.unconsumed;
    let retained_len = parser.unconsumed_len;
    let parsed = parser.parsed;

    TRACK_MEMORY.set(true);
    let baseline = LIVE_BYTES.get();
    for _ in 0..1024 {
      assert_eq!(parser.parse(invalid.as_ptr(), invalid.len()), 0);
      assert_eq!(parser.state, STATE_ERROR);
      assert_eq!(parser.error_code, code);
      assert_eq!(parser.error_description_str(), description);
      assert_eq!(parser.unconsumed_len, retained_len);
      assert_eq!(parser.unconsumed, retained);
      assert_eq!(parser.parsed, parsed);
      assert_eq!(parser.position, 0);
      assert_eq!(LIVE_BYTES.get(), baseline);
    }
    TRACK_MEMORY.set(false);

    parser.reset(false);
    let valid = b"GET / HTTP/1.1\r\n\r\n";
    assert_eq!(parser.parse(valid.as_ptr(), valid.len()), valid.len());
    assert_eq!(parser.error_code, ERROR_NONE);
  }
}

#[test]
#[allow(non_snake_case)]
fn issue_16__release_retained_input() {
  let mut parser = Parser::new();
  parser.manage_unconsumed = true;
  let prefix = b"GET / HTTP/1.1\r\nX-Test: ";
  assert_eq!(parser.parse(prefix.as_ptr(), prefix.len()), 16);
  assert_eq!(parser.unconsumed_len, 8);

  // Replacing an incomplete header must free the previous allocation, even
  // when no new input arrives.
  for input in [b"a".as_slice(), b"", b"b"] {
    let previous_len = parser.unconsumed_len;
    FREED_SIZE.store(0, Ordering::SeqCst);
    WATCHED_POINTER.store(parser.unconsumed as usize, Ordering::SeqCst);
    assert_eq!(parser.parse(input.as_ptr(), input.len()), 0);
    assert_eq!(WATCHED_POINTER.load(Ordering::SeqCst), 0);
    assert_eq!(FREED_SIZE.load(Ordering::SeqCst), previous_len);
    assert_eq!(parser.unconsumed_len, previous_len + input.len());
  }

  let previous_len = parser.unconsumed_len;
  WATCHED_POINTER.store(parser.unconsumed as usize, Ordering::SeqCst);
  let suffix = b"\r\n\r\n";
  assert_eq!(parser.parse(suffix.as_ptr(), suffix.len()), previous_len + suffix.len());
  assert_eq!(WATCHED_POINTER.load(Ordering::SeqCst), 0);
  assert_eq!(FREED_SIZE.load(Ordering::SeqCst), previous_len);
  assert!(parser.unconsumed.is_null());
  assert_eq!(parser.unconsumed_len, 0);
  assert_eq!(parser.error_code, ERROR_NONE);

  // Reset and destruction must also release pending input, including after
  // malformed input has moved the parser into its terminal error state.
  for input in [prefix.as_slice(), b"GET / HTTP/1.1\r\nX-Test: \0"] {
    for reset in [false, true] {
      parser.reset(false);
      parser.parse(input.as_ptr(), input.len());
      assert!(parser.unconsumed_len > 0);
      if input.ends_with(b"\0") {
        assert_eq!(parser.state, STATE_ERROR);
      }
      let previous_len = parser.unconsumed_len;
      WATCHED_POINTER.store(parser.unconsumed as usize, Ordering::SeqCst);
      // Disabling retention must not lose ownership of an existing buffer.
      parser.manage_unconsumed = false;
      if reset {
        parser.reset(false);
        assert!(parser.unconsumed.is_null());
        assert_eq!(parser.unconsumed_len, 0);
      } else {
        drop(parser);
        parser = Parser::new();
      }
      assert_eq!(WATCHED_POINTER.load(Ordering::SeqCst), 0);
      assert_eq!(FREED_SIZE.load(Ordering::SeqCst), previous_len);
      parser.manage_unconsumed = true;
    }
  }
}
