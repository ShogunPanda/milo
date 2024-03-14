#![feature(vec_into_raw_parts)]
#![feature(cell_update)]
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

use js_sys::{Function, Uint8Array};
use milo_macros::{
  callback, callback_no_return, generate_callbacks, generate_constants, generate_enums, init_constants,
};
use wasm_bindgen::prelude::wasm_bindgen;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::JsValue;

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

init_constants!();

mod parse;
pub use parse::parse;
mod states;

use crate::states::*;

generate_constants!();
generate_enums!();
generate_callbacks!();

// Do not change the the order of fields here, they are used to address them as
// general memory space in WebAssembly.
#[repr(C)]
#[derive(Clone, Debug)]
pub struct Parser {
  // General values
  pub state: Cell<usize>,
  pub position: Cell<usize>,
  pub error_code: Cell<usize>,
  pub error_description_len: Cell<usize>,
  pub unconsumed_len: Cell<usize>,
  pub id: Cell<usize>,
  pub mode: Cell<usize>,
  pub message_type: Cell<usize>,
  pub method: Cell<usize>,
  pub status: Cell<usize>,
  pub version_major: Cell<usize>,
  pub version_minor: Cell<usize>,
  pub connection: Cell<usize>,

  // Large values
  pub parsed: Cell<u64>,
  pub content_length: Cell<u64>,
  pub chunk_size: Cell<u64>,
  pub remaining_content_length: Cell<u64>,
  pub remaining_chunk_size: Cell<u64>,

  // Flags
  pub paused: Cell<bool>,
  pub manage_unconsumed: Cell<bool>,
  pub continue_without_data: Cell<bool>,
  pub is_connect: Cell<bool>,
  pub has_content_length: Cell<bool>,
  pub has_chunked_transfer_encoding: Cell<bool>,
  pub has_upgrade: Cell<bool>,
  pub has_trailers: Cell<bool>,
  pub skip_body: Cell<bool>,

  // Pointers
  pub offsets: Cell<*mut usize>,
  pub unconsumed: Cell<*const c_uchar>,
  pub error_description: Cell<*const c_uchar>,
  pub owner: Cell<*mut c_void>,

  // Callbacks handling
  pub callbacks: Callbacks,
  #[cfg(target_family = "wasm")]
  pub callback_error: RefCell<JsValue>,
}

/// Creates a new parser
pub fn create(id: Option<usize>) -> Parser {
  let offsets = [0; MAX_OFFSETS_COUNT].to_vec();
  let (offset_ptr, _, _) = { offsets.into_raw_parts() };

  Parser {
    // General values
    state: Cell::new(0),
    position: Cell::new(0),
    error_code: Cell::new(ERROR_NONE),
    error_description_len: Cell::new(0),
    unconsumed_len: Cell::new(0),
    id: Cell::new(id.unwrap_or(0)),
    mode: Cell::new(0),
    message_type: Cell::new(0),
    method: Cell::new(0),
    status: Cell::new(0),
    version_major: Cell::new(0),
    version_minor: Cell::new(0),
    connection: Cell::new(0),

    // Large values
    parsed: Cell::new(0),
    content_length: Cell::new(0),
    chunk_size: Cell::new(0),
    remaining_content_length: Cell::new(0),
    remaining_chunk_size: Cell::new(0),

    // Flags
    paused: Cell::new(false),
    manage_unconsumed: Cell::new(false),
    continue_without_data: Cell::new(false),
    is_connect: Cell::new(false),
    has_content_length: Cell::new(false),
    has_chunked_transfer_encoding: Cell::new(false),
    has_upgrade: Cell::new(false),
    has_trailers: Cell::new(false),
    skip_body: Cell::new(false),

    // Pointers
    offsets: Cell::new(offset_ptr),
    owner: Cell::new(ptr::null_mut()),
    unconsumed: Cell::new(ptr::null()),
    error_description: Cell::new(ptr::null()),

    // Callbacks handling
    callbacks: Callbacks::new(),
    #[cfg(target_family = "wasm")]
    callback_error: RefCell::new(JsValue::NULL),
  }
}

/// Resets a parser. The second parameters specifies if to also reset the
/// parsed counter.
pub fn reset(parser: &Parser, keep_parsed: bool) {
  parser.state.set(0);
  parser.paused.set(false);

  if !keep_parsed {
    parser.parsed.set(0);
  }

  parser.error_code.set(ERROR_NONE);
  parser.error_description.set(ptr::null());
  parser.error_description_len.set(0);

  if parser.unconsumed_len.get() > 0 {
    unsafe {
      let len = parser.unconsumed_len.get();
      Vec::from_raw_parts(parser.unconsumed.get() as *mut c_uchar, len, len);
    }

    parser.unconsumed.set(ptr::null());
    parser.unconsumed_len.set(0);
  }

  clear(parser);

  callback_no_return!(on_reset);
}

/// Clears all values in the parser.
///
/// Persisted fields, unconsumed data and the position are not cleared.
pub fn clear(parser: &Parser) {
  parser.message_type.set(0);
  parser.is_connect.set(false);
  parser.method.set(0);
  parser.status.set(0);
  parser.version_major.set(0);
  parser.version_minor.set(0);
  parser.connection.set(0);
  parser.has_content_length.set(false);
  parser.has_chunked_transfer_encoding.set(false);
  parser.has_upgrade.set(false);
  parser.has_trailers.set(false);
  parser.content_length.set(0);
  parser.chunk_size.set(0);
  parser.remaining_content_length.set(0);
  parser.remaining_chunk_size.set(0);
  parser.skip_body.set(false);
}

/// Pauses the parser. It will have to be resumed via `milo_resume`.
pub fn pause(parser: &Parser) { parser.paused.set(true); }

/// Resumes the parser.
pub fn resume(parser: &Parser) { parser.paused.set(false); }

/// Marks the parser as finished. Any new data received via `parse` will
/// put the parser in the error state.
pub fn finish(parser: &Parser) {
  match parser.state.get() {
    // If the parser is one of the initial states, simply jump to finish
    STATE_START | STATE_MESSAGE | STATE_REQUEST | STATE_RESPONSE | STATE_FINISH => {
      parser.state.set(STATE_FINISH);
    }
    STATE_BODY_WITH_NO_LENGTH => {
      // Notify that the message has been completed
      callback_no_return!(on_message_complete);

      // Set the state to be finished
      parser.state.set(STATE_FINISH);
    }
    STATE_ERROR => (),
    // In another other state, this is an error
    _ => {
      let _ = fail(parser, ERROR_UNEXPECTED_EOF, "Unexpected end of data");
    }
  }
}

// TODO@PI: Document this (Rust & WASM)
/// Clear the parser offsets.
pub fn clear_offsets(parser: &Parser) {
  unsafe {
    *(parser.offsets.get()).offset(2) = 0;
  }
}

/// Returns the current parser's state as string.
pub fn state_string(parser: &Parser) -> &str { States::try_from(parser.state.get()).unwrap().as_str() }

/// Returns the current parser's error state as string.
pub fn error_code_string(parser: &Parser) -> &str { Errors::try_from(parser.error_code.get()).unwrap().as_str() }

/// Returns the current parser's error descrition.
pub fn error_description_string(parser: &Parser) -> &str {
  unsafe {
    str::from_utf8_unchecked(from_raw_parts(
      parser.error_description.get(),
      parser.error_description_len.get(),
    ))
  }
}

/// Moves the parsers to a new state and marks a certain number of characters
/// as used.
///
/// The allow annotation is needed when building in release mode.
#[allow(dead_code)]
fn move_to(parser: &Parser, state: usize, advance: isize) -> isize {
  // Notify the end of the current state
  #[cfg(debug_assertions)]
  callback!(before_state_change);

  // Change the state
  parser.state.set(state);

  // Notify the start of the current state
  #[cfg(debug_assertions)]
  callback!(after_state_change);

  advance
}

/// Marks the parsing a failed, setting a error code and and error message.
fn fail(parser: &Parser, code: usize, reason: &str) -> isize {
  // Set the code
  parser.error_code.set(code);

  parser.error_description.set(reason.as_ptr());
  parser.error_description_len.set(reason.len());
  parser.state.set(STATE_ERROR);

  // Do not process any additional data
  0
}

#[cfg(not(target_family = "wasm"))]
mod native;

#[cfg(not(target_family = "wasm"))]
pub use crate::native::*;

#[cfg(target_family = "wasm")]
mod wasm;

#[cfg(target_family = "wasm")]
pub use crate::wasm::*;
