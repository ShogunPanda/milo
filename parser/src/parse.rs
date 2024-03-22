#![allow(clippy::not_unsafe_ptr_arg_deref)]

extern crate alloc;

use alloc::vec::Vec;
use alloc::{boxed::Box, format};
use core::cell::{Cell, RefCell};
use core::ffi::{c_char, c_uchar, c_void};
use core::fmt::Debug;
use core::ptr;
use core::str;
use core::{slice, slice::from_raw_parts};

use js_sys::{Function, Uint8Array};
use milo_macros::{callback_no_return, get, set};
#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

#[cfg(target_family = "wasm")]
use crate::run_callback;
use crate::{
  fail, Parser, ERROR_CALLBACK_ERROR, ERROR_UNEXPECTED_DATA, MAX_OFFSETS_COUNT, STATES_HANDLERS, STATE_ERROR,
  STATE_FINISH, SUSPEND, VALUE_OFFSETS_COUNT,
};

/// Parses a slice of characters. It returns the number of consumed
/// characters.
pub fn parse(parser: &Parser, data: *const c_uchar, limit: usize) -> usize {
  // If the parser is paused, this is a no-op
  if get!(paused) {
    return 0;
  }

  let data = unsafe { from_raw_parts(data, limit) };

  // Set the data to analyze, prepending unconsumed data from previous iteration
  // if needed
  let mut consumed = 0;
  let mut limit = limit;
  let aggregate: Vec<c_uchar>;
  let unconsumed_len = get!(unconsumed_len);

  let mut current = if get!(manage_unconsumed) && unconsumed_len > 0 {
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
  let mut previous_state = get!(state);

  // Since states might advance position manually, the parser have to explicitly
  // track it
  set!(position, 0);
  let mut initial_position = 0;

  let offsets = &parser.offsets;
  unsafe { parser.values.add(VALUE_OFFSETS_COUNT).cast::<usize>().write(0) };

  // Until there is data or there is a request to continue
  while !current.is_empty() || get!(continue_without_data) {
    // Reset the continue_without_data bit
    set!(continue_without_data, false);

    // If the parser has finished and it receives more data, error
    if get!(state) == STATE_FINISH {
      let _ = fail(parser, ERROR_UNEXPECTED_DATA, "unexpected data");
      continue;
    }

    // Apply the current state
    let result = (STATES_HANDLERS[get!(state)])(parser, current);
    let new_state = get!(state);

    // If the parser finished or errored, execute callbacks
    if new_state == STATE_FINISH {
      callback_no_return!(on_finish);
    } else if new_state == STATE_ERROR {
      callback_no_return!(on_error);
      break;
    } else if unsafe { parser.values.add(VALUE_OFFSETS_COUNT).cast::<usize>().read() } == MAX_OFFSETS_COUNT {
      // We can't write a new offset, bail out earlier
      break;
    } else if result == SUSPEND {
      // If the state suspended the parser, then bail out earlier
      break;
    }

    // Update the position of the parser
    let new_position = get!(position) + (result as usize);
    set!(position, new_position);

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
          get!(state),
          duration
        );
      }

      last = Instant::now();
      previous_state = new_state;
    }

    // If a callback paused the parser, break now
    if get!(paused) {
      break;
    }
  }

  set!(parsed, get!(parsed) + (consumed as u64));

  if get!(manage_unconsumed) {
    unsafe {
      // Drop any previous retained data
      if unconsumed_len > 0 {
        Vec::from_raw_parts(parser.unconsumed.get() as *mut c_uchar, unconsumed_len, unconsumed_len);

        parser.unconsumed.set(ptr::null());
        set!(unconsumed_len, 0);
      }

      // If less bytes were consumed than requested, copy the unconsumed portion in
      // the parser for the next iteration
      if consumed < limit {
        let (ptr, len, _) = current.to_vec().into_raw_parts();

        parser.unconsumed.set(ptr);
        set!(unconsumed_len, len);
      }
    }
  }

  #[cfg(all(debug_assertions, feature = "debug"))]
  {
    let duration = Instant::now().duration_since(start).as_nanos();

    if duration > 0 {
      println!(
        "[milo::debug] parse ({:?}, consumed {} of {}) completed in {} ns",
        get!(state),
        consumed,
        limit,
        duration
      );
    }
  }

  unsafe {
    *(offsets.offset(0)) = get!(state);
    *(offsets.offset(1)) = consumed;
  }

  // Return the number of consumed bytes
  consumed
}
