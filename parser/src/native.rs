#![allow(clippy::not_unsafe_ptr_arg_deref)]

use core::ptr;
use core::str;
use core::{slice, slice::from_raw_parts};
use std::ffi::{c_char, c_uchar, CString};

use crate::flags;
use crate::parse;
use crate::Flags;
use crate::Parser;

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

impl From<&str> for CStringWithLength {
  fn from(value: &str) -> Self { CStringWithLength::new(value) }
}

impl From<CStringWithLength> for &str {
  fn from(value: CStringWithLength) -> Self {
    unsafe { str::from_utf8_unchecked(slice::from_raw_parts(value.ptr, value.len)) }
  }
}

/// A callback that simply returns `0`.
///
/// Use this callback as pointer when you want to remove a callback from the
/// parser.
#[no_mangle]
pub extern "C" fn milo_noop(_parser: &mut Parser, _at: usize, _len: usize) {}

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

/// Destroys a parser.
#[no_mangle]
pub extern "C" fn milo_destroy(parser: *mut Parser) {
  if parser.is_null() {
    return;
  }

  unsafe {
    let _ = Box::from_raw(parser);
  }
}

/// Parses a slice of characters. It returns the number of consumed characters.
#[no_mangle]
pub extern "C" fn milo_parse(parser: *mut Parser, data: *const c_uchar, limit: usize) -> usize {
  unsafe { (*parser).parse(data, limit) }
}

/// Resets a parser. The second parameters specifies if to also reset the
/// parsed counter.
///
/// The following fields are not modified:
///   * position
///   * context
///   * mode
///   * manage_unconsumed
///   * continue_without_data
///   * context
#[no_mangle]
pub extern "C" fn milo_reset(parser: *mut Parser, keep_parsed: bool) { unsafe { (*parser).reset(keep_parsed) } }

/// Clears all values about the message in the parser.
///
/// The connection and message type fields are not cleared.  
#[no_mangle]
pub extern "C" fn milo_clear(parser: *mut Parser) { unsafe { (*parser).clear() } }

/// Pauses the parser. It will have to be resumed via `milo_resume`.
#[no_mangle]
pub extern "C" fn milo_pause(parser: *mut Parser) { unsafe { (*parser).pause() } }

/// Resumes the parser.
#[no_mangle]
pub extern "C" fn milo_resume(parser: *mut Parser) { unsafe { (*parser).resume() } }

/// Marks the parser as finished. Any new data received via `milo_parse` will
/// put the parser in the error state.
#[no_mangle]
pub extern "C" fn milo_finish(parser: *mut Parser) { unsafe { (*parser).finish() } }

/// Marks the parsing a failed, setting a error code and and error message.
#[no_mangle]
pub extern "C" fn milo_fail(parser: *mut Parser, code: usize, description: CStringWithLength) {
  unsafe { (*parser).fail(code, description.into()) };
}

/// Returns the current parser's state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_state_string(parser: *mut Parser) -> CStringWithLength {
  unsafe { (*parser).state_str().into() }
}

/// Returns the current parser's error state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_error_code_string(parser: *mut Parser) -> CStringWithLength {
  unsafe { (*parser).error_code_str().into() }
}

/// Returns the current parser's error descrition.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_error_description_string(parser: *mut Parser) -> CStringWithLength {
  unsafe { (*parser).error_description_str().into() }
}
