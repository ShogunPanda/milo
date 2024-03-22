#![allow(clippy::not_unsafe_ptr_arg_deref)]

use core::ptr;
use core::str;
use core::{slice, slice::from_raw_parts};
use std::ffi::{c_char, c_uchar, CString};

use milo_macros::callback_no_return;
use milo_macros::native_getters;

use crate::clear;
use crate::clear_offsets;
use crate::create;
use crate::error_code_string;
use crate::error_description_string;
use crate::fail;
use crate::finish;
use crate::flags;
use crate::parse;
use crate::pause;
use crate::reset;
use crate::resume;
use crate::set_is_connect;
use crate::set_manage_unconsumed;
use crate::set_mode;
use crate::set_skip_body;
use crate::state_string;
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

impl Into<&str> for CStringWithLength {
  fn into(self) -> &'static str { unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.ptr, self.len)) } }
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
pub extern "C" fn milo_create() -> *mut Parser { Box::into_raw(Box::new(create(None))) }

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

/// Resets a parser to its initial state.
#[no_mangle]
pub extern "C" fn milo_reset(parser: *const Parser, keep_parsed: bool) { unsafe { reset(&*parser, keep_parsed) } }

/// Clears all values in the parser.
///
/// Persisted fields, unconsumed data and the position are not cleared.
#[no_mangle]
pub extern "C" fn milo_clear(parser: *const Parser) { unsafe { clear(&*parser) } }

// TODO@PI: Document this (ALL)
/// Clear the parser offsets.
#[no_mangle]
pub extern "C" fn milo_clear_offsets(parser: *const Parser) { unsafe { clear_offsets(&*parser) } }

/// Parses a slice of characters. It returns the number of consumed characters.
#[no_mangle]
pub extern "C" fn milo_parse(parser: *const Parser, data: *const c_uchar, limit: usize) -> usize {
  unsafe { parse(&*parser, data, limit) }
}

/// Pauses the parser. It will have to be resumed via `milo_resume`.
#[no_mangle]
pub extern "C" fn milo_pause(parser: *const Parser) { unsafe { pause(&*parser) } }

/// Resumes the parser.
#[no_mangle]
pub extern "C" fn milo_resume(parser: *const Parser) { unsafe { resume(&*parser) } }

/// Marks the parser as finished. Any new data received via `milo_parse` will
/// put the parser in the error state.
#[no_mangle]
pub extern "C" fn milo_finish(parser: *const Parser) { unsafe { finish(&*parser) } }

/// Marks the parsing a failed, setting a error code and and error message.
#[no_mangle]
pub extern "C" fn milo_fail(parser: &Parser, code: usize, reason: CStringWithLength) {
  fail(&*parser, code, reason.into());
}

native_getters!();

// Sets the parser mode.
#[no_mangle]
pub extern "C" fn milo_set_mode(parser: &Parser, value: usize) { set_mode(parser, value); }

// Sets whether the parser should manage unconsumed data.
#[no_mangle]
pub extern "C" fn milo_set_manage_unconsumed(parser: &Parser, value: bool) { set_manage_unconsumed(parser, value); }

// Sets whether the parser should skip the body.
#[no_mangle]
pub extern "C" fn milo_set_skip_body(parser: &Parser, value: bool) { set_skip_body(parser, value); }

// Sets if the request is a connect request.
#[no_mangle]
pub extern "C" fn milo_set_is_connect(parser: &Parser, value: bool) { set_is_connect(parser, value); }

/// Returns the current parser's state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_state_string(parser: *const Parser) -> CStringWithLength {
  unsafe { CStringWithLength::new(state_string(&*parser)) }
}

/// Returns the current parser's error state as string.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_error_code_string(parser: *const Parser) -> CStringWithLength {
  unsafe { CStringWithLength::new(error_code_string(&*parser)) }
}

/// Returns the current parser's error descrition.
///
/// The returned value must be freed using `free_string`.
#[no_mangle]
pub extern "C" fn milo_error_description_string(parser: *const Parser) -> CStringWithLength {
  unsafe { CStringWithLength::new(error_description_string(&*parser)) }
}
