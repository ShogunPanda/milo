#![allow(unused_imports)]

extern crate alloc;

use alloc::ffi::CString;
use alloc::vec::Vec;
use alloc::{boxed::Box, format};
use core::cell::{Cell, RefCell};
use core::ffi::{c_char, c_uchar, c_void};
use core::fmt::Debug;
use core::ptr;
use core::slice;
use core::str;

use milo_macros::generate;

const NO_INCOMPLETE_LINE: u8 = u8::MAX;

#[repr(C)]
#[derive(Debug)]
pub struct Parser {
  // User writable
  #[cfg(not(target_family = "wasm"))]
  pub context: *mut c_void,
  pub max_start_line_length: usize,
  pub max_field_length: usize,
  pub max_fields_bytes: usize,
  pub max_fields_count: usize,
  pub max_body_payload: u64,
  pub autodetect: bool,
  pub is_request: bool,
  pub suspend_after_headers: bool,
  pub manage_unconsumed: bool,
  pub continue_without_data: bool,
  pub is_connect: bool,
  pub skip_body: bool,
  pub debug: bool,

  // Generic state
  pub parsed: u64,
  pub position: usize,
  pub state: u8,
  pub paused: bool,
  pub error_code: u8,
  /// Requires exactly one non-empty initial Host field in HTTP/1.1 requests.
  pub require_host: bool,

  // Current message flags
  pub content_length: u64,
  pub chunk_size: u64,
  pub remaining_content_length: u64,
  pub remaining_chunk_size: u64,
  pub status: u32,
  pub method: u8,
  pub has_content_length: bool,
  pub has_transfer_encoding: bool,
  pub has_chunked_transfer_encoding: bool,
  pub has_connection_close: bool,
  pub has_connection_upgrade: bool,
  pub has_upgrade: bool,
  pub has_trailers: bool,

  // Callback handling
  pub active_callbacks: u64,
  pub active_events: u64,
  #[cfg(not(target_family = "wasm"))]
  pub callbacks: ParserCallbacks,

  // WASM Specific
  #[cfg(target_family = "wasm")]
  pub ptr: *mut c_void,

  // Complex data types - We need to split them in order to be exportable to C++
  error_description: [u8; 255],
  unconsumed: *const c_uchar,
  unconsumed_len: usize,
  error_description_len: u8,

  // Internal parser state
  parsing: bool,
  has_pending_start: bool,
  incomplete_line_state: u8,
  fields_bytes: usize,
  fields_count: usize,
  retained_capacity: usize,
  has_host: bool,
  pending_start_at: u64,

  // The managed aggregate backing the latest parse.
  current_input: *const c_uchar,
  current_input_len: usize,
  retained_offset: usize,

  // Event buffer. Keep this at the end of the struct for external readers.
  events: *mut c_uchar,
}

#[cfg(not(target_family = "wasm"))]
mod native;

#[cfg(not(target_family = "wasm"))]
pub use crate::native::*;

#[cfg(target_family = "wasm")]
mod wasm;

#[cfg(target_family = "wasm")]
pub use crate::wasm::*;

generate!();

impl Parser {
  /// Creates a new parser
  pub fn new() -> Parser {
    let events = Box::new([0u8; EVENTS_BUFFER_SIZE]);
    let events_ptr = Box::into_raw(events).cast();

    Parser {
      // User writable
      #[cfg(not(target_family = "wasm"))]
      context: ptr::null_mut(),
      max_start_line_length: 8192,
      max_field_length: 8192,
      max_fields_bytes: 64 * 1024,
      max_fields_count: 100,
      max_body_payload: 0,
      autodetect: true,
      is_request: false,
      suspend_after_headers: false,
      manage_unconsumed: false,
      continue_without_data: false,
      is_connect: false,
      skip_body: false,
      debug: false,
      // Generic state
      parsed: 0,
      position: 0,
      state: STATE_START,
      paused: false,
      error_code: ERROR_NONE,
      require_host: false,
      // Current message flags
      content_length: 0,
      chunk_size: 0,
      remaining_content_length: 0,
      remaining_chunk_size: 0,
      status: 0,
      method: 0,
      has_content_length: false,
      has_transfer_encoding: false,
      has_chunked_transfer_encoding: false,
      has_connection_close: false,
      has_connection_upgrade: false,
      has_upgrade: false,
      has_trailers: false,
      // Callbacks handling
      active_callbacks: 0,
      active_events: 0,
      #[cfg(not(target_family = "wasm"))]
      callbacks: ParserCallbacks::new(),
      // WASM Specific
      #[cfg(target_family = "wasm")]
      ptr: ptr::null_mut(),
      // Complex data types
      error_description: [0; 255],
      unconsumed: ptr::null(),
      unconsumed_len: 0,
      error_description_len: 0,
      // Internal parser state
      parsing: false,
      has_pending_start: false,
      incomplete_line_state: NO_INCOMPLETE_LINE,
      fields_bytes: 0,
      fields_count: 0,
      retained_capacity: 0,
      has_host: false,
      pending_start_at: 0,
      current_input: ptr::null(),
      current_input_len: 0,
      retained_offset: 0,
      events: events_ptr,
    }
  }

  /// Resets a parser. The second parameters specifies if to also reset the
  /// parsed counter.
  ///
  /// The following fields are not modified:
  ///   * position
  ///   * context
  ///   * autodetect
  ///   * is_request
  ///   * suspend_after_headers
  ///   * require_host
  ///   * manage_unconsumed
  ///   * continue_without_data
  ///   * max_fields_bytes
  ///   * max_fields_count
  ///   * debug
  ///   * context
  pub fn reset(&mut self, keep_parsed: bool) {
    if self.parsing {
      return;
    }

    self.state = STATE_START;
    self.paused = false;

    if !keep_parsed {
      self.parsed = 0;
    }

    self.error_code = ERROR_NONE;

    self.error_description[0] = 0;
    self.error_description_len = 0;

    self.clear_current_input();
    self.clear_unconsumed();

    self.clear();
    self.skip_body = false;
    self.has_pending_start = false;
    // SAFETY: The parser owns the event buffer and its first byte.
    unsafe { *self.events = EVENT_END };
  }

  pub(crate) fn clear_unconsumed(&mut self) {
    if self.retained_capacity != 0 {
      drop(self.take_retained_input());
    }
  }

  fn take_retained_input(&mut self) -> Vec<u8> {
    self.incomplete_line_state = NO_INCOMPLETE_LINE;
    let capacity = core::mem::take(&mut self.retained_capacity);
    let data = core::mem::replace(&mut self.unconsumed, ptr::null()).cast_mut();
    let len = core::mem::take(&mut self.unconsumed_len);
    let offset = core::mem::take(&mut self.retained_offset);
    // SAFETY: These are the original Vec parts, now owned by the returned Vec.
    unsafe { Vec::from_raw_parts(data.sub(offset), len + offset, capacity) }
  }

  fn put_retained_input(&mut self, data: Vec<u8>, incomplete_line_state: u8) {
    let mut data = core::mem::ManuallyDrop::new(data);
    self.unconsumed = data.as_mut_ptr();
    self.unconsumed_len = data.len();
    self.retained_capacity = data.capacity();
    self.incomplete_line_state = incomplete_line_state;
  }

  fn append_retained_input(&mut self, input: &[u8]) {
    if input.is_empty() && self.retained_offset == 0 {
      return;
    }
    let incomplete_line_state = self.incomplete_line_state;
    let offset = self.retained_offset;
    let mut retained = self.take_retained_input();
    if offset != 0 {
      retained.copy_within(offset.., 0);
      retained.truncate(retained.len() - offset);
    }
    retained.extend_from_slice(input);
    self.put_retained_input(retained, incomplete_line_state);
  }

  fn keep_retained_suffix(&mut self, input: &[u8], consumed: usize, incomplete_line_state: u8) {
    if self.unconsumed_len == 0 {
      self.put_retained_input(input[consumed..].to_vec(), incomplete_line_state);
      return;
    }

    let len = self.unconsumed_len - consumed;
    if consumed != 0 {
      // SAFETY: Both ranges are within the retained allocation and may overlap.
      unsafe { ptr::copy(self.unconsumed.add(consumed), self.unconsumed.cast_mut(), len) };
    }
    self.unconsumed_len = len;
    self.incomplete_line_state = incomplete_line_state;
  }

  #[cold]
  #[inline(never)]
  fn clear_current_input(&mut self) {
    if self.retained_offset != 0 && self.unconsumed_len == 0 {
      self.clear_unconsumed();
    }
    self.current_input = ptr::null();
    self.current_input_len = 0;
  }

  fn retain_event_aggregate(&mut self, retain: bool, consumed: usize, incomplete_line_state: u8) {
    // Keep the aggregate intact and expose only the pending suffix as unconsumed.
    let offset = if retain { consumed } else { self.unconsumed_len };
    // SAFETY: Offset is within, or just past, the existing allocation.
    self.unconsumed = unsafe { self.unconsumed.add(offset) };
    self.unconsumed_len -= offset;
    self.retained_offset = offset;
    self.incomplete_line_state = incomplete_line_state;
  }

  /// Clears all values about the message in the parser.
  pub fn clear(&mut self) {
    if self.parsing {
      return;
    }

    self.clear_message();
  }

  fn clear_message(&mut self) {
    self.is_connect = false;
    self.method = 0;
    self.status = 0;
    self.has_content_length = false;
    self.has_transfer_encoding = false;
    self.has_chunked_transfer_encoding = false;
    self.has_connection_close = false;
    self.has_connection_upgrade = false;
    self.has_upgrade = false;
    self.has_trailers = false;
    self.content_length = 0;
    self.chunk_size = 0;
    self.remaining_content_length = 0;
    self.remaining_chunk_size = 0;
    self.fields_bytes = 0;
    self.fields_count = 0;
    self.has_host = false;
  }

  #[inline(always)]
  pub(crate) fn try_emit_event_range(&mut self, event_cursor: &mut usize, event_type: u8, at: u32, len: u32) -> bool {
    // Reserve six bytes for a later error event and one for the terminator.
    if *event_cursor + EVENT_RANGE_SIZE + EVENT_ERROR_RESERVE > EVENTS_BUFFER_SIZE {
      return false;
    }

    // SAFETY: The capacity check covers the entire range event.
    unsafe {
      *self.events.add(*event_cursor) = event_type;
      core::ptr::write_unaligned(self.events.add(*event_cursor + 1) as *mut u32, at.to_le());
      core::ptr::write_unaligned(self.events.add(*event_cursor + 5) as *mut u32, len.to_le());
    }
    *event_cursor += EVENT_RANGE_SIZE;
    true
  }

  #[inline(always)]
  pub(crate) fn try_emit_event_error(&mut self, event_cursor: &mut usize, position: u32) -> bool {
    if *event_cursor + EVENT_ERROR_SIZE + EVENT_END_SIZE > EVENTS_BUFFER_SIZE {
      return false;
    }

    // SAFETY: The capacity check covers the entire error event.
    unsafe {
      *self.events.add(*event_cursor) = EVENT_ERROR;
      core::ptr::write_unaligned(self.events.add(*event_cursor + 1) as *mut u32, position.to_le());
      *self.events.add(*event_cursor + 5) = self.error_code;
    }
    *event_cursor += EVENT_ERROR_SIZE;
    true
  }

  /// Returns the current event batch.
  ///
  /// Records are terminated by [`EVENT_END`]. Parser-owned storage cannot be
  /// replaced through the safe Rust API:
  ///
  /// ```compile_fail
  /// use milo_parser::Parser;
  ///
  /// let mut parser = Parser::new();
  /// parser.events = core::ptr::null_mut();
  /// ```
  pub fn events(&self) -> &[u8; EVENTS_BUFFER_SIZE] {
    // SAFETY: The parser owns this array for its lifetime.
    unsafe { &*self.events.cast::<[u8; EVENTS_BUFFER_SIZE]>() }
  }

  /// Returns input retained for the next managed parse call.
  pub fn unconsumed(&self) -> &[u8] {
    if self.unconsumed_len == 0 {
      &[]
    } else {
      // SAFETY: The parser owns this allocation and safe Rust cannot replace
      // its pointer or length.
      unsafe { slice::from_raw_parts(self.unconsumed, self.unconsumed_len) }
    }
  }

  /// Returns the aggregate backing the latest parse, or an empty slice if no
  /// retained prefix was prepended.
  ///
  /// # Safety
  /// Use the slice only before the next parse, reset, complete, finish, or fail.
  pub unsafe fn current_input(&self) -> &[u8] {
    if self.current_input.is_null() {
      return &[];
    }
    // SAFETY: The parser owns this allocation until it invalidates the view.
    unsafe { slice::from_raw_parts(self.current_input, self.current_input_len) }
  }

  /// Pauses the parser. It will have to be resumed via `resume`.
  pub fn pause(&mut self) { self.paused = true; }

  /// Resumes the parser.
  pub fn resume(&mut self) { self.paused = false; }

  #[cold]
  #[inline(never)]
  fn remember_pending_start(&mut self, data: &[u8]) {
    if self.autodetect && !self.paused && (data == b"\r" || (data.len() < 5 && b"HTTP".starts_with(data))) {
      // A complete prefix or message advances the stream past this offset.
      // An empty parse must not overwrite an outstanding prefix.
      self.pending_start_at = self.parsed.wrapping_add(self.position as u64);
      self.has_pending_start = true;
    }
  }

  /// Marks the parser as finished. Any new data received via `parse` will
  /// put the parser in the error state.
  pub fn finish(&mut self) {
    if self.parsing {
      return;
    }

    self.clear_current_input();
    match self.state {
      STATE_START => {
        if self.has_pending_start && self.pending_start_at == self.parsed {
          self.fail(ERROR_UNEXPECTED_EOF, "Unexpected end of data");
        } else {
          self.state = STATE_FINISH;
        }
      }
      STATE_FINISH => (),
      STATE_BODY_WITH_NO_LENGTH => {
        let active_events = self.active_events | self.active_callbacks;
        let mut event_cursor = 0usize;
        if let Ok(position) = u32::try_from(self.position) {
          if active_events & EVENT_ACTIVE_ON_MESSAGE_COMPLETE != 0 {
            self.try_emit_event_range(&mut event_cursor, EVENT_MESSAGE_COMPLETE, position, 0);
          }
          if active_events & EVENT_ACTIVE_ON_FINISH != 0 {
            self.try_emit_event_range(&mut event_cursor, EVENT_FINISH, position, 0);
          }
        }
        // SAFETY: The cursor follows at most two range events in the owned buffer.
        unsafe {
          *self.events.add(event_cursor) = EVENT_END;
        }

        self.state = STATE_FINISH;
        self.replay_control_callbacks();
      }
      STATE_ERROR => (),
      // In another other state, this is an error
      _ => {
        self.fail(ERROR_UNEXPECTED_EOF, "Unexpected end of data");
      }
    }
  }

  /// Marks parsing as failed and starts a standalone error event batch.
  #[inline(always)]
  pub fn fail(&mut self, code: u8, description: &str) {
    if self.parsing {
      return;
    }

    self.clear_current_input();
    self.set_error(code, description);
    let mut event_cursor = 0usize;
    if (self.active_events | self.active_callbacks) & EVENT_ACTIVE_ON_ERROR != 0
      && let Ok(position) = u32::try_from(self.position)
    {
      self.try_emit_event_error(&mut event_cursor, position);
    }
    // SAFETY: The cursor follows at most one error event in the owned event buffer.
    unsafe {
      *self.events.add(event_cursor) = EVENT_END;
    }
    self.replay_control_callbacks();
  }

  fn replay_control_callbacks(&mut self) {
    if self.active_callbacks != 0 {
      self.parsing = true;
      self.invoke_callbacks();
      self.parsing = false;
    }
  }

  /// Records an error without changing the current event batch.
  #[inline(always)]
  pub(crate) fn set_error(&mut self, code: u8, description: &str) {
    let bytes = description.as_bytes();
    let mut len = bytes.len().min(254);
    while !description.is_char_boundary(len) {
      len -= 1;
    }

    self.state = STATE_ERROR;
    self.error_code = code;
    self.error_description[..len].copy_from_slice(&bytes[..len]);
    self.error_description[len] = 0;
    self.error_description_len = len as u8;
  }

  /// Returns the current parser's state as string.
  pub fn state_str(&self) -> &str { States::try_from(self.state).unwrap().as_str() }

  /// Returns the current parser's error state as string.
  pub fn error_code_str(&self) -> &str { Errors::try_from(self.error_code).unwrap().as_str() }

  /// Returns the current parser's error description as string.
  pub fn error_description_str(&self) -> &str {
    if self.error_description_len == 0 {
      return "";
    }

    str::from_utf8(&self.error_description[..self.error_description_len as usize]).unwrap_or("")
  }
}

impl Drop for Parser {
  fn drop(&mut self) {
    self.clear_current_input();
    self.clear_unconsumed();
    if !self.events.is_null() {
      // SAFETY: This is the original pointer returned by Box::into_raw.
      unsafe {
        drop(Box::from_raw(self.events.cast::<[u8; EVENTS_BUFFER_SIZE]>()));
      }
      self.events = ptr::null_mut();
    }
  }
}

impl Default for Parser {
  fn default() -> Self { Self::new() }
}

mod matchers;
mod parse;
