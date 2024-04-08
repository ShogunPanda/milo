#![feature(vec_into_raw_parts)]
#![feature(cell_update)]
#![feature(exposed_provenance)]
#![allow(unused_imports)]

extern crate alloc;

use alloc::ffi::CString;
use alloc::vec::Vec;
use alloc::{boxed::Box, format};
use core::cell::{Cell, RefCell};
use core::ffi::{c_char, c_uchar, c_void};
use core::fmt::Debug;
use core::ptr;
use core::str;
use core::{slice, slice::from_raw_parts};

use milo_macros::{
  callback, generate_callbacks, generate_constants, generate_enums, init_constants, link_callbacks, r#return,
};

init_constants!();

#[cfg(target_family = "wasm")]
#[link(wasm_import_module = "env")]
extern "C" {
  link_callbacks!();

  #[cfg(debug_assertions)]
  fn logger(message: u64);
}

#[cfg(all(debug_assertions, target_family = "wasm"))]
#[no_mangle]
pub fn __start() {
  std::panic::set_hook(Box::new(|panic_info| {
    debug(format!("WebAssembly panicked: {:#?}", panic_info));
  }));
}

#[repr(C)]
pub struct Flags {
  pub debug: bool,
}

#[no_mangle]
pub fn flags() -> Flags {
  Flags {
    debug: cfg!(debug_assertions),
  }
}

mod states;

use crate::states::*;

generate_constants!();
generate_enums!();
generate_callbacks!();

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Parser {
  // User writable
  pub mode: u8,
  pub manage_unconsumed: bool,
  pub continue_without_data: bool,
  pub is_connect: bool,
  pub skip_body: bool,
  #[cfg(not(target_family = "wasm"))]
  pub context: *mut c_void,

  // Generic state
  pub state: u8,
  pub position: usize,
  pub parsed: u64,
  pub paused: bool,
  pub error_code: u8,

  // Current message flags
  pub message_type: u8,
  pub method: u8,
  pub status: u32,
  pub version_major: u8,
  pub version_minor: u8,
  pub connection: u8,
  pub content_length: u64,
  pub chunk_size: u64,
  pub remaining_content_length: u64,
  pub remaining_chunk_size: u64,
  pub has_content_length: bool,
  pub has_chunked_transfer_encoding: bool,
  pub has_upgrade: bool,
  pub has_trailers: bool,

  // Callback handling
  #[cfg(not(target_family = "wasm"))]
  pub callbacks: ParserCallbacks,

  // WASM Specific
  #[cfg(target_family = "wasm")]
  pub ptr: *mut c_void,

  // Complex data types - We need to split them in order to be exportable to C++
  pub error_description: *const c_uchar,
  pub error_description_len: u16,
  pub unconsumed: *const c_uchar,
  pub unconsumed_len: usize,
}

impl Default for Parser {
  fn default() -> Self { Self::new() }
}

impl Parser {
  /// Creates a new parser
  pub fn new() -> Parser {
    Parser {
      // User writable
      mode: MESSAGE_TYPE_AUTODETECT,
      manage_unconsumed: false,
      continue_without_data: false,
      is_connect: false,
      skip_body: false,
      #[cfg(not(target_family = "wasm"))]
      context: ptr::null_mut(),
      // Generic state
      state: STATE_START,
      position: 0,
      parsed: 0,
      paused: false,
      error_code: ERROR_NONE,
      // Current message flags
      message_type: 0,
      method: 0,
      status: 0,
      version_major: 0,
      version_minor: 0,
      connection: 0,
      content_length: 0,
      chunk_size: 0,
      remaining_content_length: 0,
      remaining_chunk_size: 0,
      has_content_length: false,
      has_chunked_transfer_encoding: false,
      has_upgrade: false,
      has_trailers: false,
      // Callbacks handling
      #[cfg(not(target_family = "wasm"))]
      callbacks: ParserCallbacks::new(),
      // WASM Specific
      #[cfg(target_family = "wasm")]
      ptr: ptr::null_mut(),
      // Complex data types
      error_description: ptr::null(),
      error_description_len: 0,
      unconsumed: ptr::null(),
      unconsumed_len: 0,
    }
  }

  /// Resets a parser. The second parameters specifies if to also reset the
  /// parsed counter.
  ///
  /// The following fields are not modified:
  ///   * position
  ///   * context
  ///   * mode
  ///   * manage_unconsumed
  ///   * continue_without_data
  ///   * context
  pub fn reset(&mut self, keep_parsed: bool) {
    self.state = STATE_START;
    self.paused = false;

    if !keep_parsed {
      self.parsed = 0;
    }

    self.message_type = 0;
    self.connection = 0;
    self.error_code = ERROR_NONE;

    if self.error_description_len > 0 {
      unsafe {
        let _ = slice::from_raw_parts(self.error_description, self.error_description_len as usize);
      }

      self.error_description = ptr::null();
      self.error_description_len = 0;
    }

    if self.unconsumed_len > 0 {
      unsafe {
        let _ = slice::from_raw_parts(self.unconsumed, self.unconsumed_len);
      }

      self.unconsumed = ptr::null();
      self.unconsumed_len = 0;
    }

    callback!(on_reset);
  }

  /// Clears all values about the message in the parser.
  ///
  /// The connection and message type fields are not cleared.  
  pub fn clear(&mut self) {
    self.is_connect = false;
    self.method = 0;
    self.status = 0;
    self.version_major = 0;
    self.version_minor = 0;
    self.has_content_length = false;
    self.has_chunked_transfer_encoding = false;
    self.has_upgrade = false;
    self.has_trailers = false;
    self.content_length = 0;
    self.chunk_size = 0;
    self.remaining_content_length = 0;
    self.remaining_chunk_size = 0;
    self.skip_body = false;
  }

  /// Pauses the parser. It will have to be resumed via `resume`.
  pub fn pause(&mut self) { self.paused = true; }

  /// Resumes the parser.
  pub fn resume(&mut self) { self.paused = false; }

  /// Marks the parser as finished. Any new data received via `parse` will
  /// put the parser in the error state.
  pub fn finish(&mut self) {
    match self.state {
      // If the parser is one of the initial states, simply jump to finish
      STATE_START | STATE_AUTODETECT | STATE_REQUEST | STATE_RESPONSE | STATE_FINISH => {
        self.state = STATE_FINISH;
      }
      STATE_BODY_WITH_NO_LENGTH => {
        // Notify that the message has been completed
        callback!(on_message_complete);

        // Set the state to be finished
        self.state = STATE_FINISH;
      }
      STATE_ERROR => (),
      // In another other state, this is an error
      _ => {
        self.fail(ERROR_UNEXPECTED_EOF, "Unexpected end of data");
      }
    }
  }

  /// Moves the parsers to a new state and marks a certain number of characters
  /// as used.
  ///
  /// The allow annotation is needed when building in release mode.
  #[allow(dead_code)]
  pub fn move_to(&mut self, state: u8, advance: usize) {
    // Change the state
    self.state = state;
    self.position += advance;
  }

  /// Marks the parsing a failed, setting a error code and and error message.
  ///
  /// It always returns zero for internal use.
  pub fn fail(&mut self, code: u8, description: &str) {
    let description_copy = description.to_string();
    let (ptr, _, len) = description_copy.into_raw_parts();

    self.state = STATE_ERROR;
    self.error_code = code;
    self.error_description = ptr;
    self.error_description_len = len as u16;
  }

  /// Returns the current parser's state as string.
  pub fn state_str(&self) -> &str { States::try_from(self.state).unwrap().as_str() }

  /// Returns the current parser's error state as string.
  pub fn error_code_str(&self) -> &str { Errors::try_from(self.error_code).unwrap().as_str() }

  /// Returns the current parser's error description as string.
  pub fn error_description_str(&self) -> &str {
    unsafe {
      if self.error_description_len > 0 {
        str::from_utf8_unchecked(from_raw_parts(
          self.error_description,
          self.error_description_len as usize,
        ))
      } else {
        ""
      }
    }
  }
}

mod parse;

#[cfg(not(target_family = "wasm"))]
mod native;

#[cfg(not(target_family = "wasm"))]
pub use crate::native::*;

#[cfg(target_family = "wasm")]
mod wasm;

#[cfg(target_family = "wasm")]
pub use crate::wasm::*;
