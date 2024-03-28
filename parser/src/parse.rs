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

use milo_macros::{callback, callback_on_self, callback_on_self_no_return};

#[cfg(target_family = "wasm")]
use crate::run_callback;
use crate::{Parser, ERROR_UNEXPECTED_DATA, STATES_HANDLERS, STATE_ERROR, STATE_FINISH, SUSPEND};

impl Parser {
  /// Parses a slice of characters.
  ///
  /// It returns the number of consumed characters.
  pub fn parse(&mut self, data: *const c_uchar, limit: usize) -> usize {
    // If the self.is paused, this is a no-op
    if self.paused {
      return 0;
    }

    let data = unsafe { from_raw_parts(data, limit) };

    // Set the data to analyze, prepending unconsumed data from previous iteration
    // if needed
    let mut consumed = 0;
    let mut limit = limit;
    let aggregate: Vec<c_uchar>;
    let unconsumed_len = self.unconsumed_len;

    let mut current = if self.manage_unconsumed && unconsumed_len > 0 {
      unsafe {
        limit += unconsumed_len;
        let unconsumed = from_raw_parts(self.unconsumed, unconsumed_len);

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
    let mut previous_state = self.state;

    // States will advance position manually, the parser has to explicitly
    // track it
    self.position = 0;
    let mut initial_position = 0;

    // Until there is data or there is a request to continue
    while !current.is_empty() || self.continue_without_data {
      // Reset the continue_without_data bit
      self.continue_without_data = false;

      // Apply the current state
      let result = (STATES_HANDLERS[self.state])(self, current);
      let new_state = self.state;

      // If the self.finished or errored, execute callbacks
      if new_state == STATE_FINISH {
        callback_on_self_no_return!(on_finish);
      } else if new_state == STATE_ERROR {
        callback_on_self_no_return!(on_error);
        break;
      } else if result == SUSPEND {
        // If the state suspended the self. then bail out earlier
        break;
      }

      // Update the position of the parser
      self.position += result;

      // Compute how many bytes were actually consumed and then advance the data
      let difference = self.position - initial_position;
      consumed += difference;
      current = &current[difference..];
      initial_position = self.position;

      // Show the duration of the operation if asked to
      #[cfg(all(debug_assertions, feature = "debug"))]
      {
        let duration = Instant::now().duration_since(last).as_nanos();

        if duration > 0 {
          println!(
            "[milo::debug] loop iteration ({:?} -> {:?}) completed in {} ns",
            previous_state, self.state, duration
          );
        }

        last = Instant::now();
        previous_state = new_state;
      }

      // If a callback paused the self. break now
      if self.paused {
        break;
      }
    }

    self.parsed += consumed as u64;

    if self.manage_unconsumed {
      unsafe {
        // Drop any previous retained data
        if unconsumed_len > 0 {
          let _ = from_raw_parts(self.unconsumed, unconsumed_len);
        }

        // If less bytes were consumed than requested, copy the unconsumed portion in
        // the self.for the next iteration
        if consumed < limit {
          let (ptr, len, _) = current.to_vec().into_raw_parts();

          self.unconsumed = ptr;
          self.unconsumed_len = len;
        } else {
          self.unconsumed = ptr::null();
          self.unconsumed_len = 0;
        }
      }
    }

    #[cfg(all(debug_assertions, feature = "debug"))]
    {
      let duration = Instant::now().duration_since(start).as_nanos();

      if duration > 0 {
        println!(
          "[milo::debug] parse ({:?}, consumed {} of {}) completed in {} ns",
          self.state, consumed, limit, duration
        );
      }
    }

    // Return the number of consumed bytes
    consumed
  }
}
