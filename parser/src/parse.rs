#![allow(clippy::not_unsafe_ptr_arg_deref)]

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
use milo_macros::callback_no_return;
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::JsValue;

use crate::{
  fail, states_handlers, Parser, ERROR_CALLBACK_ERROR, ERROR_UNEXPECTED_DATA, STATE_ERROR, STATE_FINISH, SUSPEND,
};

/// Parses a slice of characters. It returns the number of consumed
/// characters.
pub fn parse(parser: &Parser, data: *const c_uchar, limit: usize) -> usize {
  // If the parser is paused, this is a no-op

  if parser.paused.get() {
    return 0;
  }

  let data = unsafe { from_raw_parts(data, limit) };

  // Set the data to analyze, prepending unconsumed data from previous iteration
  // if needed
  let mut consumed = 0;
  let mut limit = limit;
  let aggregate: Vec<c_uchar>;
  let unconsumed_len = parser.unconsumed_len.get();

  let mut current = if parser.manage_unconsumed.get() && unconsumed_len > 0 {
    unsafe {
      limit += unconsumed_len;
      let unconsumed = from_raw_parts(parser.unconsumed.get(), unconsumed_len);

      aggregate = [unconsumed, data].concat();
      &aggregate[..]
    }
  } else {
    data
  };

  // Limit the data that is currently analyzed
  current = &current[..limit];

  #[cfg(all(debug_assertions, feature = "debug"))]
  let mut last = Instant::now();

  #[cfg(all(debug_assertions, feature = "debug"))]
  let start = Instant::now();

  #[cfg(all(debug_assertions, feature = "debug"))]
  let mut previous_state = parser.state.get();

  // Since states might advance position manually, the parser have to explicitly
  // track it
  let mut initial_position = parser.position.update(|_| 0);

  let offsets = parser.offsets.get();
  unsafe { *(offsets.offset(2)) = 0 };

  // Until there is data or there is a request to continue
  while !current.is_empty() || parser.continue_without_data.get() {
    // Reset the continue_without_data bit
    parser.continue_without_data.set(false);

    // If the parser has finished and it receives more data, error
    if parser.state.get() == STATE_FINISH {
      let _ = fail(parser, ERROR_UNEXPECTED_DATA, "unexpected data");
      continue;
    }

    // Apply the current state
    let result = (states_handlers[parser.state.get()])(parser, current);
    let new_state = parser.state.get();

    // If the parser finished or errored, execute callbacks
    if new_state == STATE_FINISH {
      callback_no_return!(on_finish);
    } else if new_state == STATE_ERROR {
      callback_no_return!(on_error);
      break;
    } else if result == SUSPEND {
      // If the state suspended the parser, then bail out earlier
      break;
    }

    // Update the position of the parser
    let new_position = parser.position.update(|x| x + (result as usize));

    // Compute how many bytes were actually consumed and then advance the data
    let difference = new_position - initial_position;

    consumed += difference;
    current = &current[difference..];
    initial_position = new_position;

    // Show the duration of the operation if asked to
    #[cfg(all(debug_assertions, feature = "debug"))]
    {
      let duration = Instant::now().duration_since(last).as_nanos();

      if duration > 0 {
        println!(
          "[milo::debug] loop iteration ({:?} -> {:?}) completed in {} ns",
          previous_state,
          parser.state.get(),
          duration
        );
      }

      last = Instant::now();
      previous_state = new_state;
    }

    // If a callback paused the parser, break now
    if parser.paused.get() {
      break;
    }
  }

  parser.parsed.update(|x| x + (consumed as u64));

  if parser.manage_unconsumed.get() {
    unsafe {
      // Drop any previous retained data
      if unconsumed_len > 0 {
        Vec::from_raw_parts(parser.unconsumed.get() as *mut c_uchar, unconsumed_len, unconsumed_len);

        parser.unconsumed.set(ptr::null());
        parser.unconsumed_len.set(0);
      }

      // If less bytes were consumed than requested, copy the unconsumed portion in
      // the parser for the next iteration
      if consumed < limit {
        let (ptr, len, _) = current.to_vec().into_raw_parts();

        parser.unconsumed.set(ptr);
        parser.unconsumed_len.set(len);
      }
    }
  }

  #[cfg(all(debug_assertions, feature = "debug"))]
  {
    let duration = Instant::now().duration_since(start).as_nanos();

    if duration > 0 {
      println!(
        "[milo::debug] parse ({:?}, consumed {} of {}) completed in {} ns",
        parser.state.get(),
        consumed,
        limit,
        duration
      );
    }
  }

  unsafe {
    *(offsets.offset(0)) = parser.state.get();
    *(offsets.offset(1)) = consumed;
  }

  // Return the number of consumed bytes
  consumed
}
