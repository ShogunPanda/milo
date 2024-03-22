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
  callback, callback_no_return, generate_callbacks, generate_constants, generate_enums, get, getters, init_constants,
  set,
};
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

#[cfg(target_family = "wasm")]
#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_name = runCallback, catch)]
  fn run_callback(parser: *mut c_void, callback: usize, data: usize, limit: usize) -> Result<isize, JsValue>;
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

init_constants!();

mod parse;
pub use parse::parse;
mod states;

use crate::states::*;

generate_constants!();
generate_enums!();
generate_callbacks!();

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Parser {
  // Pointers
  pub values: *mut usize,
  pub offsets: *mut usize,

  // TODO@PI: Revisit this and the presence of the *_len in values.yml
  pub unconsumed: Cell<*const c_uchar>,
  pub error_description: Cell<*const c_uchar>,

  // TODO@PI: Make this only writeable in the constructor so you don't need a cell
  #[cfg(not(target_family = "wasm"))]
  pub owner: Cell<*mut c_void>,
  #[cfg(target_family = "wasm")]
  pub ptr: Cell<*mut c_void>,

  // Callbacks handling
  #[cfg(not(target_family = "wasm"))]
  pub callbacks: CallbacksRegistry,
  #[cfg(target_family = "wasm")]
  pub callback_error: RefCell<JsValue>,
}

/// Creates a new parser
pub fn create(id: Option<usize>) -> Parser {
  let values = Vec::with_capacity(VALUES_SIZE);
  let (values_ptr, _, _) = { values.into_raw_parts() };

  let offsets = Vec::with_capacity(MAX_OFFSETS_COUNT * 3);
  let (offset_ptr, _, _) = { offsets.into_raw_parts() };

  let parser = Parser {
    values: values_ptr,
    offsets: offset_ptr,
    #[cfg(not(target_family = "wasm"))]
    owner: Cell::new(ptr::null_mut()),
    unconsumed: Cell::new(ptr::null()),
    error_description: Cell::new(ptr::null()),
    #[cfg(target_family = "wasm")]
    ptr: Cell::new(ptr::null_mut()),

    // Callbacks handling
    #[cfg(not(target_family = "wasm"))]
    callbacks: CallbacksRegistry::new(),
    #[cfg(target_family = "wasm")]
    callback_error: RefCell::new(JsValue::NULL),
  };

  unsafe { parser.values.add(VALUE_ID).cast::<usize>().write(id.unwrap_or(0)) };
  parser
}

/// Resets a parser. The second parameters specifies if to also reset the
/// parsed counter.
///
/// The following fields are not modified:
///   * position
///   * id
///   * owner
///   * mode
///   * manage_unconsumed
///   * continue_without_data
pub fn reset(parser: &Parser, keep_parsed: bool) {
  set!(state, 0);
  set!(paused, false);

  if !keep_parsed {
    set!(parsed, 0);
  }

  set!(error_code, ERROR_NONE);
  parser.error_description.set(ptr::null());
  set!(error_description_len, 0);

  if get!(unconsumed_len) > 0 {
    unsafe {
      let len = get!(unconsumed_len);
      Vec::from_raw_parts(parser.unconsumed.get() as *mut c_uchar, len, len);
    }

    parser.unconsumed.set(ptr::null());
    set!(unconsumed_len, 0);
  }

  set!(message_type, 0);
  set!(connection, 0);
  clear(parser);

  callback_no_return!(on_reset);

  #[cfg(target_family = "wasm")]
  parser.callback_error.replace(JsValue::NULL);
}

/// Clears all values about the message in the parser.
///
/// The connection and message type fields are not cleared.  
pub fn clear(parser: &Parser) {
  set!(is_connect, false);
  set!(method, 0);
  set!(status, 0);
  set!(version_major, 0);
  set!(version_minor, 0);
  set!(has_content_length, false);
  set!(has_chunked_transfer_encoding, false);
  set!(has_upgrade, false);
  set!(has_trailers, false);
  set!(content_length, 0);
  set!(chunk_size, 0);
  set!(remaining_content_length, 0);
  set!(remaining_chunk_size, 0);
  set!(skip_body, false);
}

// TODO@PI: Document this (Rust & WASM)
/// Clear the parser offsets.
pub fn clear_offsets(parser: &Parser) {
  unsafe {
    parser.values.add(VALUE_OFFSETS_COUNT).cast::<usize>().write(0);
  }
}

/// Pauses the parser. It will have to be resumed via `milo_resume`.
pub fn pause(parser: &Parser) {
  set!(paused, true);
}

/// Resumes the parser.
pub fn resume(parser: &Parser) {
  set!(paused, false);
}

/// Marks the parser as finished. Any new data received via `parse` will
/// put the parser in the error state.
pub fn finish(parser: &Parser) {
  match get!(state) {
    // If the parser is one of the initial states, simply jump to finish
    STATE_START | STATE_AUTODETECT | STATE_REQUEST | STATE_RESPONSE | STATE_FINISH => {
      set!(state, STATE_FINISH);
    }
    STATE_BODY_WITH_NO_LENGTH => {
      // Notify that the message has been completed
      callback_no_return!(on_message_complete);

      // Set the state to be finished
      set!(state, STATE_FINISH);
    }
    STATE_ERROR => (),
    // In another other state, this is an error
    _ => {
      let _ = fail(parser, ERROR_UNEXPECTED_EOF, "Unexpected end of data");
    }
  }
}

/// Moves the parsers to a new state and marks a certain number of characters
/// as used.
///
/// The allow annotation is needed when building in release mode.
#[allow(dead_code)]
pub fn move_to(parser: &Parser, state: usize, advance: isize) -> isize {
  // Notify the end of the current state
  #[cfg(debug_assertions)]
  callback!(before_state_change);

  // Change the state
  set!(state, state);

  // Notify the start of the current state
  #[cfg(debug_assertions)]
  callback!(after_state_change);

  advance
}

/// Marks the parsing a failed, setting a error code and and error message.
pub fn fail(parser: &Parser, code: usize, reason: &str) -> isize {
  // Set the code
  set!(error_code, code);

  parser.error_description.set(reason.as_ptr());
  set!(error_description_len, reason.len());
  set!(state, STATE_ERROR);

  // Do not process any additional data
  0
}

getters!();

// Sets the parser mode.
pub fn set_mode(parser: &Parser, value: usize) {
  unsafe { parser.values.add(VALUE_MODE).cast::<usize>().write(value) };
}

// Sets whether the parser should manage unconsumed data.
pub fn set_manage_unconsumed(parser: &Parser, value: bool) {
  unsafe { parser.values.add(VALUE_MANAGE_UNCONSUMED).cast::<bool>().write(value) };
}

// Sets whether the parser should skip the body.
pub fn set_skip_body(parser: &Parser, value: bool) {
  unsafe { parser.values.add(VALUE_SKIP_BODY).cast::<bool>().write(value) };
}

// Sets if the request is a connect request.
pub fn set_is_connect(parser: &Parser, value: bool) {
  unsafe { parser.values.add(VALUE_IS_CONNECT).cast::<bool>().write(value) };
}

/// Returns the current parser's state as string.
pub fn state_string(parser: &Parser) -> &str { States::try_from(get!(state)).unwrap().as_str() }

/// Returns the current parser's error state as string.
pub fn error_code_string(parser: &Parser) -> &str { Errors::try_from(get!(error_code)).unwrap().as_str() }

/// Returns the current parser's error descrition.
pub fn error_description_string(parser: &Parser) -> &str {
  if get!(error_description_len) == 0 {
    return "";
  }

  unsafe {
    str::from_utf8_unchecked(from_raw_parts(
      parser.error_description.get(),
      get!(error_description_len),
    ))
  }
}

#[cfg(not(target_family = "wasm"))]
mod native;

#[cfg(not(target_family = "wasm"))]
pub use crate::native::*;

#[cfg(target_family = "wasm")]
mod wasm;

#[cfg(target_family = "wasm")]
pub use crate::wasm::*;
