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
#[cfg(all(debug_assertions, feature = "debug"))]
use std::time::Instant;

use milo_macros::*;

#[cfg(target_family = "wasm")]
use crate::run_callback;
use crate::*;
use crate::{Parser, ERROR_UNEXPECTED_DATA, STATE_ERROR, STATE_FINISH, SUSPEND};

impl Parser {
  /// Parses a slice of characters.
  ///
  /// It returns the number of consumed characters.
  pub fn parse(&mut self, input: *const c_uchar, limit: usize) -> usize {
    // If the self.is paused, this is a no-op
    if self.paused {
      return 0;
    }

    let input = unsafe { from_raw_parts(input, limit) };

    // Set the data to analyze, prepending unconsumed data from previous iteration
    // if needed
    let mut limit = limit;
    let aggregate: Vec<c_uchar>;
    let unconsumed_len = self.unconsumed_len;

    let mut data = if self.manage_unconsumed && unconsumed_len > 0 {
      unsafe {
        limit += unconsumed_len;
        let unconsumed = from_raw_parts(self.unconsumed, unconsumed_len);

        aggregate = [unconsumed, input].concat();
        &aggregate[..]
      }
    } else {
      input
    };

    // Limit the data that is currently analyzed
    data = &data[..limit];
    let mut available = data.len();

    #[cfg(all(debug_assertions, feature = "debug"))]
    let mut last = Instant::now();

    #[cfg(all(debug_assertions, feature = "debug"))]
    let start = Instant::now();

    #[cfg(debug_assertions)]
    let mut previous_state = self.state;

    // States will advance position manually, the parser has to explicitly
    // track it
    self.position = 0;
    let mut last_position = 0;
    let initial_available = available;

    // Until there is data or there is a request to continue
    'parser: loop {
      #[cfg(debug_assertions)]
      if previous_state != self.state {
        callback!(on_state_change);
      }

      if self.paused || (available == 0 && !self.continue_without_data) {
        break;
      }

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
      }

      #[cfg(debug_assertions)]
      {
        previous_state = self.state;
      }

      // Reset the continue_without_data flag
      self.continue_without_data = false;

      // Advance the data
      if last_position != self.position {
        let difference = self.position - last_position;
        data = &data[difference..];
        available -= difference;
        last_position += difference;
      }

      process_state!();
    }

    let consumed = initial_available - available;
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
          let (ptr, len, _) = data.to_vec().into_raw_parts();

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
