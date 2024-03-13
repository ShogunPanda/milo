use proc_macro::TokenStream;
use quote::quote;

/// Core parser logic, it is shared amongst all the possible implementations of
/// the parsing method.
///
/// Note that this could have been achieved via #[inline(always)] and recursion,
/// but we want to make sure the compiler cannot ignore it.
pub fn parse() -> TokenStream {
  TokenStream::from(quote! {
    // Set the data to analyze, prepending unconsumed data from previous iteration
    // if needed

    let mut consumed = 0;
    let mut limit = limit;
    let aggregate: Vec<c_uchar>;
    let unconsumed_len = self.unconsumed_len.get();

    let mut current = if self.manage_unconsumed.get() && unconsumed_len > 0 {
      unsafe {
        limit += unconsumed_len;
        let unconsumed = from_raw_parts(self.unconsumed.get(), unconsumed_len);

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
    let mut previous_state = self.state.get();

    // Since states might advance position manually, the parser have to explicitly track it
    let mut initial_position = self.position.update(|_| 0);

    let offsets = self.offsets.get();
    unsafe { *(offsets.offset(2)) = 0 };

    // Until there is data or there is a request to continue
    while !current.is_empty() || self.continue_without_data.get() {
      // Reset the continue_without_data bit
      self.continue_without_data.set(false);

      // If the parser has finished and it receives more data, error
      if self.state.get() == STATE_FINISH {
        let _ = self.fail(ERROR_UNEXPECTED_DATA, "unexpected data");
        continue;
      }

      // Apply the current state
      let result = (states_handlers[self.state.get() as usize])(self, current);
      let new_state = self.state.get();

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
      let new_position = self.position.update(|x| x + (result as usize));

      // Compute how many bytes were actually consumed and then advance the data
      let difference = (new_position - initial_position) as usize;

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
            previous_state, self.state.get(), duration
          );
        }

        last = Instant::now();
        previous_state = new_state;
      }

      // If a callback paused the parser, break now
      if self.paused.get() {
        break;
      }
    }

    self.parsed.update(|x| x + (consumed as u64));

    if self.manage_unconsumed.get() {
      unsafe {
        // Drop any previous retained data
        if unconsumed_len > 0 {
          Vec::from_raw_parts(self.unconsumed.get() as *mut c_uchar, unconsumed_len, unconsumed_len);

          self.unconsumed.set(ptr::null());
          self.unconsumed_len.set(0);
        }

        // If less bytes were consumed than requested, copy the unconsumed portion in
        // the parser for the next iteration
        if consumed < limit {
          let (ptr, len, _) = current.to_vec().into_raw_parts();

          self.unconsumed.set(ptr);
          self.unconsumed_len.set(len);
        }
      }
    }

    #[cfg(all(debug_assertions, feature = "debug"))]
    {
      let duration = Instant::now().duration_since(start).as_nanos();

      if duration > 0 {
        println!(
          "[milo::debug] parse ({:?}, consumed {} of {}) completed in {} ns", self.state.get(), consumed, limit, duration
        );
      }
    }

    unsafe {
      *(offsets.offset(0)) = self.state.get() as usize;
      *(offsets.offset(1)) = consumed;
    }
  })
}
