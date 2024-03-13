use core::ptr;
use core::str;
use core::{slice, slice::from_raw_parts};
use std::ffi::{c_char, c_uchar, CString};

use milo_macros::{callback_no_return, parse};

use crate::*;

#[repr(C)]
pub struct CStringWithLength {
  pub ptr: *const c_uchar,
  pub len: usize,
}

impl CStringWithLength {
  fn new(value: &str) -> CStringWithLength {
    let cstring = CString::new(value).unwrap();

    CStringWithLength {
      ptr: cstring.into_raw() as *const c_uchar,
      len: value.len(),
    }
  }
}

// This impl only contains the parse method which cannot be exported to WASM
impl Parser {
  /// # Safety
  ///
  /// Parses a slice of characters. It returns the number of consumed
  /// characters.
  #[cfg(not(target_family = "wasm"))]
  pub fn parse(&self, data: *const c_uchar, limit: usize) -> usize {
    // If the parser is paused, this is a no-op

    if self.paused.get() {
      return 0;
    }

    let data = unsafe { from_raw_parts(data, limit) };

    parse!();

    // Return the number of consumed bytes
    consumed
  }

  /// Returns the current parser's state as string.
  pub fn state_string(&self) -> &str { States::try_from(self.state.get()).unwrap().as_str() }

  /// Returns the current parser's error state as string.
  pub fn error_code_string(&self) -> &str { Errors::try_from(self.error_code.get()).unwrap().as_str() }

  /// Returns the current parser's error descrition.
  pub fn error_description_string(&self) -> &str {
    unsafe {
      str::from_utf8_unchecked(from_raw_parts(
        self.error_description.get(),
        self.error_description_len.get(),
      ))
    }
  }
}

/// A callback that simply returns `0`.
///
/// Use this callback as pointer when you want to remove a callback from the
/// parser.
#[no_mangle]
pub extern "C" fn milo_noop(_parser: &Parser, _data: *const c_uchar, _len: usize) -> isize { 0 }

/// Return current compile flags for milo
#[no_mangle]
pub extern "C" fn milo_flags() -> Flags { flags() }

/// Cleans up memory used by a string previously returned by one of the milo's C
/// public interface.
#[no_mangle]
pub extern "C" fn milo_free_string(s: CStringWithLength) {
  unsafe {
    let _ = CString::from_raw(s.ptr as *mut c_char);
  }
}

/// Creates a new parser.
#[no_mangle]
pub extern "C" fn milo_create() -> *mut Parser { Box::into_raw(Box::new(Parser::new())) }

/// # Safety
///
/// Destroys a parser.
#[no_mangle]
pub extern "C" fn milo_destroy(ptr: *mut Parser) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Box::from_raw(ptr);
  }
}

/// # Safety
///
/// Resets a parser to its initial state.
#[no_mangle]
pub extern "C" fn milo_reset(parser: *const Parser, keep_parsed: bool) { unsafe { (*parser).reset(keep_parsed) } }

/// # Safety
///
/// Parses a slice of characters. It returns the number of consumed characters.
#[no_mangle]
pub extern "C" fn milo_parse(parser: *const Parser, data: *const c_uchar, limit: usize) -> usize {
  unsafe { (*parser).parse(data, limit) }
}

/// # Safety
///
/// Pauses the parser. It will have to be resumed via `milo_resume`.
#[no_mangle]
pub extern "C" fn milo_pause(parser: *const Parser) { unsafe { (*parser).pause() } }

/// # Safety
///
/// Resumes the parser.
#[no_mangle]
pub extern "C" fn milo_resume(parser: *const Parser) { unsafe { (*parser).resume() } }

/// # Safety
///
/// Marks the parser as finished. Any new data received via `milo_parse` will
/// put the parser in the error state.
#[no_mangle]
pub extern "C" fn milo_finish(parser: *const Parser) { unsafe { (*parser).finish() } }

// TODO@PI: Document this (ALL)
/// # Safety
// Clear the offsets
#[no_mangle]
pub extern "C" fn clear_offsets(parser: *const Parser) { unsafe { (*parser).clear_offsets() } }

/// # Safety
///
/// Returns the current parser's state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_state_string(parser: *const Parser) -> CStringWithLength {
  unsafe { CStringWithLength::new((*parser).state_string()) }
}

/// # Safety
///
/// Returns the current parser's error state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_error_code_string(parser: *const Parser) -> CStringWithLength {
  unsafe { CStringWithLength::new((*parser).error_code_string()) }
}

/// # Safety
///
/// Returns the current parser's error descrition.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_error_description_string(parser: *const Parser) -> CStringWithLength {
  unsafe { CStringWithLength::new((*parser).error_description_string()) }
}
