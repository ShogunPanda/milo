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
use milo_macros::*;
use wasm_bindgen::prelude::*;

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

mod states;
use crate::states::*;

generate_constants!();
generate_enums!();
generate_callbacks!();

#[wasm_bindgen]
#[repr(C)]
#[cfg_attr(not(target_family = "wasm"), derive(Clone, Debug))]
pub struct Parser {
  #[wasm_bindgen(skip)]
  pub owner: Cell<*mut c_void>,

  #[wasm_bindgen(skip)]
  pub state: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub position: Cell<usize>,

  #[wasm_bindgen(skip)]
  pub parsed: Cell<u64>,

  #[wasm_bindgen(skip)]
  pub paused: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub error_code: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub error_description: Cell<*const c_uchar>,

  #[wasm_bindgen(skip)]
  pub error_description_len: Cell<usize>,

  #[wasm_bindgen(skip)]
  pub unconsumed: Cell<*const c_uchar>,

  #[wasm_bindgen(skip)]
  pub unconsumed_len: Cell<usize>,

  #[wasm_bindgen(skip)]
  pub id: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub mode: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub manage_unconsumed: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub continue_without_data: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub message_type: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub is_connect: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub method: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub status: Cell<usize>,

  #[wasm_bindgen(skip)]
  pub version_major: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub version_minor: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub connection: Cell<u8>,

  #[wasm_bindgen(skip)]
  pub has_content_length: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub has_chunked_transfer_encoding: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub has_upgrade: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub has_trailers: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub content_length: Cell<u64>,

  #[wasm_bindgen(skip)]
  pub chunk_size: Cell<u64>,

  #[wasm_bindgen(skip)]
  pub remaining_content_length: Cell<u64>,

  #[wasm_bindgen(skip)]
  pub remaining_chunk_size: Cell<u64>,

  #[wasm_bindgen(skip)]
  pub skip_body: Cell<bool>,

  #[wasm_bindgen(skip)]
  pub callbacks: Callbacks,

  #[wasm_bindgen(skip)]
  pub offsets: Cell<*mut usize>,

  #[cfg(target_family = "wasm")]
  #[wasm_bindgen(skip)]
  pub callback_error: RefCell<JsValue>,
}

#[wasm_bindgen]
impl Parser {
  pub fn new() -> Parser {
    let offsets = [0; MAX_OFFSETS_COUNT].to_vec();
    let (offset_ptr, _, _) = { offsets.into_raw_parts() };

    Parser {
      owner: Cell::new(ptr::null_mut()),
      state: Cell::new(0),
      position: Cell::new(0),
      parsed: Cell::new(0),
      paused: Cell::new(false),
      error_code: Cell::new(ERROR_NONE),
      error_description: Cell::new(ptr::null()),
      error_description_len: Cell::new(0),
      unconsumed: Cell::new(ptr::null()),
      unconsumed_len: Cell::new(0),
      id: Cell::new(0),
      mode: Cell::new(0),
      manage_unconsumed: Cell::new(false),
      continue_without_data: Cell::new(false),
      message_type: Cell::new(0),
      is_connect: Cell::new(false),
      method: Cell::new(0),
      status: Cell::new(0),
      version_major: Cell::new(0),
      version_minor: Cell::new(0),
      connection: Cell::new(0),
      has_content_length: Cell::new(false),
      has_chunked_transfer_encoding: Cell::new(false),
      has_upgrade: Cell::new(false),
      has_trailers: Cell::new(false),
      content_length: Cell::new(0),
      chunk_size: Cell::new(0),
      remaining_content_length: Cell::new(0),
      remaining_chunk_size: Cell::new(0),
      skip_body: Cell::new(false),
      callbacks: Callbacks::new(),
      offsets: Cell::new(offset_ptr),
      #[cfg(target_family = "wasm")]
      callback_error: RefCell::new(JsValue::NULL),
    }
  }

  /// Resets a parser. The second parameters specifies if to also reset the
  /// parsed counter.
  #[wasm_bindgen]
  pub fn reset(&self, keep_parsed: bool) {
    self.state.set(0);
    self.paused.set(false);

    if !keep_parsed {
      self.parsed.set(0);
    }

    self.error_code.set(ERROR_NONE);
    self.error_description.set(ptr::null());
    self.error_description_len.set(0);

    if self.unconsumed_len.get() > 0 {
      unsafe {
        let len = self.unconsumed_len.get();
        Vec::from_raw_parts(self.unconsumed.get() as *mut c_uchar, len, len);
      }

      self.unconsumed.set(ptr::null());
      self.unconsumed_len.set(0);
    }

    self.clear();

    callback_no_return!(on_reset);
  }

  /// Clears all values in the parser.
  ///
  /// Persisted fields, unconsumed data and the position are not cleared.
  #[wasm_bindgen]
  pub fn clear(&self) {
    self.message_type.set(0);
    self.is_connect.set(false);
    self.method.set(0);
    self.status.set(0);
    self.version_major.set(0);
    self.version_minor.set(0);
    self.connection.set(0);
    self.has_content_length.set(false);
    self.has_chunked_transfer_encoding.set(false);
    self.has_upgrade.set(false);
    self.has_trailers.set(false);
    self.content_length.set(0);
    self.chunk_size.set(0);
    self.remaining_content_length.set(0);
    self.remaining_chunk_size.set(0);
    self.skip_body.set(false);
  }

  /// # Safety
  ///
  /// Parses a slice of characters. It returns the number of consumed
  /// characters.

  /// Moves the parsers to a new state and marks a certain number of characters
  /// as used.
  ///
  /// The allow annotation is needed when building in release mode.
  #[allow(dead_code)]
  fn move_to(&self, state: u8, advance: isize) -> isize {
    let parser = self;

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
  fn fail(&self, code: u8, reason: &str) -> isize {
    // Set the code
    self.error_code.set(code);

    self.error_description.set(reason.as_ptr());
    self.error_description_len.set(reason.len());
    self.state.set(STATE_ERROR);

    // Do not process any additional data
    0
  }

  /// Pauses the parser. It will have to be resumed via `milo_resume`.
  #[wasm_bindgen]
  pub fn pause(&self) { self.paused.set(true); }

  /// Resumes the parser.
  #[wasm_bindgen]
  pub fn resume(&self) { self.paused.set(false); }

  /// Marks the parser as finished. Any new data received via `parse` will
  /// put the parser in the error state.
  #[wasm_bindgen]
  pub fn finish(&self) {
    match self.state.get() {
      // If the parser is one of the initial states, simply jump to finish
      STATE_START | STATE_MESSAGE | STATE_REQUEST | STATE_RESPONSE | STATE_FINISH => {
        self.state.set(STATE_FINISH);
      }
      STATE_BODY_WITH_NO_LENGTH => {
        // Notify that the message has been completed
        callback_no_return!(on_message_complete);

        // Set the state to be finished
        self.state.set(STATE_FINISH);
      }
      STATE_ERROR => (),
      // In another other state, this is an error
      _ => {
        let _ = self.fail(ERROR_UNEXPECTED_EOF, "Unexpected end of data");
      }
    }
  }

  // TODO@PI: Document this (Rust & WASM)
  // Clear the offsets
  #[wasm_bindgen(js_name = clearOffsets)]
  pub fn clear_offsets(&self) {
    unsafe {
      *(self.offsets.get()).offset(2) = 0;
    }
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
