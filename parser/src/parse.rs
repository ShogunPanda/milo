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
    let mut previous_position = 0;
    let initial_available = available;
    let mut not_suspended = true;

    #[cfg(all(debug_assertions, feature = "debug"))]
    eprintln!("[milo::debug] loop enter");

    // Until there is data or there is a request to continue
    'parser: while not_suspended
      && (!self.paused || self.state == STATE_COMPLETE)
      && (available != 0 || self.continue_without_data)
    {
      #[cfg(all(debug_assertions, feature = "debug"))]
      {
        eprintln!(
          "[milo::debug] loop before processing: previous_position={}, position={}, available={}, \
           continue_without_data={}",
          previous_position, self.position, available, self.continue_without_data
        );
      }

      // Reset the continue_without_data flag
      self.continue_without_data = false;

      'state: {
        process_state!();
      }

      // Update the parser position
      if previous_position != self.position {
        let difference = self.position - previous_position;
        data = &data[difference..];
        available -= difference;

        #[cfg(all(debug_assertions, feature = "debug"))]
        {
          eprintln!(
            "[milo::debug] loop before processing: previous_position={}, position={}, difference={}, available={}, \
             continue_without_data={}",
            previous_position, self.position, difference, available, self.continue_without_data
          );
        }

        previous_position += difference;
      }

      // Notify the status change
      #[cfg(debug_assertions)]
      if previous_state != self.state {
        callback!(on_state_change);
        previous_state = self.state;
      }

      // Show the duration of the operation
      #[cfg(all(debug_assertions, feature = "debug"))]
      {
        let duration = Instant::now().duration_since(last).as_nanos();

        if duration > 0 {
          eprintln!(
            "[milo::debug] loop iteration ({:?}) completed in {} ns",
            self.state_str(),
            duration
          );
        }

        last = Instant::now();
      }
    }

    #[cfg(all(debug_assertions, feature = "debug"))]
    eprintln!("[milo::debug] loop exit");

    if previous_position != self.position {
      let difference = self.position - previous_position;
      available -= difference;
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
        eprintln!(
          "[milo::debug] parse ({:?}, consumed {} of {}) completed in {} ns",
          self.state_str(),
          consumed,
          limit,
          duration
        );
      }
    }

    // Return the number of consumed bytes
    consumed
  }
}
