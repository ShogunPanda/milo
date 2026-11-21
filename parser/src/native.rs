#![allow(clippy::not_unsafe_ptr_arg_deref)]

use core::{slice, str};
use std::ffi::{CString, c_char, c_uchar};

use crate::parse;
use crate::{Callbacks, Errors, Events, Methods, Parser, States};

#[repr(C)]
pub struct CStringWithLength {
  pub ptr: *const c_uchar,
  pub len: usize,
}

impl CStringWithLength {
  fn new(value: &str) -> CStringWithLength {
    let bytes = value.as_bytes();
    let len = bytes.iter().position(|byte| *byte == 0).unwrap_or(bytes.len());
    let cstring = CString::new(&bytes[..len]).expect("bytes before the first NUL cannot contain a NUL");

    CStringWithLength {
      ptr: cstring.into_raw() as *const c_uchar,
      len,
    }
  }
}

impl From<&str> for CStringWithLength {
  fn from(value: &str) -> Self { CStringWithLength::new(value) }
}

/// Returns if debug informations are available in this build.
#[unsafe(no_mangle)]
pub extern "C" fn milo_has_debug() -> bool { cfg!(any(debug_assertions, feature = "debug")) }

/// Returns if debug tracing is enabled for this parser.
///
/// # Safety
/// `parser` must point to a live parser.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_is_debug(parser: *const Parser) -> bool { unsafe { (*parser).debug } }

/// A callback that simply returns `0`.
///
/// Use this callback as pointer when you want to remove a callback from the
/// parser.
#[unsafe(no_mangle)]
pub extern "C" fn milo_noop(_parser: &mut Parser, _at: usize, _len: usize) {}

/// Cleans up memory used by a string previously returned by one of the milo's C
/// public interface.
///
/// # Safety
/// `s` must have been returned by Milo and must not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_free_string(s: CStringWithLength) {
  unsafe {
    let _ = CString::from_raw(s.ptr as *mut c_char);
  }
}

/// Creates a new parser.
#[unsafe(no_mangle)]
pub extern "C" fn milo_create() -> *mut Parser { Box::into_raw(Box::new(Parser::new())) }

/// Destroys a parser. During callback replay this is a no-op.
///
/// # Safety
/// `parser` must be null or a live pointer returned by [`milo_create`]. The
/// caller must have exclusive access except during its callback replay.
/// Do not use the parser after successful destruction.
///
/// Calling raw FFI helpers requires an explicit unsafe block:
///
/// ```compile_fail
/// use milo_parser::{milo_create, milo_destroy};
///
/// let parser = milo_create();
/// milo_destroy(parser);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_destroy(parser: *mut Parser) {
  // SAFETY: A non-null parser must be live by contract.
  if parser.is_null() || unsafe { (*parser).parsing } {
    return;
  }

  // SAFETY: The parser is owned by the caller and is not in replay.
  unsafe {
    let _ = Box::from_raw(parser);
  }
}

/// Parses a byte range. It returns the number of consumed bytes.
///
/// # Safety
/// `parser` must point to a live, properly aligned parser that the caller can
/// exclusively access for the duration of the call.
/// If `limit` is nonzero, `data` must point to `limit` initialized bytes in one
/// allocation, with `limit <= isize::MAX`. The bytes must stay valid and
/// unchanged and must not overlap the parser or its owned buffers. If `limit`
/// is zero, `data` may be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_parse(parser: *mut Parser, data: *const c_uchar, limit: usize) -> usize {
  // SAFETY: The caller provides a live parser.
  if unsafe { (*parser).parsing } {
    return 0;
  }

  // SAFETY: The caller supplies valid pointers and the parser is not replaying.
  unsafe { parse_ffi_body(parser, data, limit) }
}

#[inline(never)]
unsafe extern "C" fn parse_ffi_body(parser: *mut Parser, data: *const c_uchar, limit: usize) -> usize {
  if limit > u32::MAX as usize {
    // SAFETY: The FFI contract guarantees exclusive access to a live parser.
    let parser = unsafe { &mut *parser };
    if parser.paused || parser.state == crate::STATE_ERROR {
      return parser.parse_body(&[]);
    }
    return parser.reject_oversized_input("Input exceeds the 32-bit event coordinate limit");
  }

  let input = if limit == 0 {
    &[]
  } else {
    // SAFETY: Required by this function's contract.
    unsafe { slice::from_raw_parts(data, limit) }
  };
  // SAFETY: Required by this function's contract.
  unsafe { (*parser).parse_body(input) }
}

/// Sets the parser event bitmask.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_set_active_events(parser: *mut Parser, value: u64) {
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).active_events = value;
    }
  }
}

/// Sets the maximum body payload consumed by a single parse invocation.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_set_max_body_payload(parser: *mut Parser, value: u64) {
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).max_body_payload = value;
    }
  }
}

/// Sets whether parsing should stop after headers have completed.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_set_suspend_after_headers(parser: *mut Parser, value: bool) {
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).suspend_after_headers = value;
    }
  }
}

/// Resets a parser. The second parameters specifies if to also reset the
/// parsed counter.
///
/// The following fields are not modified:
///   * position
///   * context
///   * autodetect
///   * is_request
///   * suspend_after_headers
///   * max_body_payload
///   * manage_unconsumed
///   * continue_without_data
///   * debug
///   * context
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_reset(parser: *mut Parser, keep_parsed: bool) {
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).reset(keep_parsed);
    }
  }
}

/// Clears all values about the message in the parser.
///
/// The autodetect and is_request fields are not cleared.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_clear(parser: *mut Parser) {
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).clear();
    }
  }
}

/// Pauses the parser. It will have to be resumed via `milo_resume`.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_pause(parser: *mut Parser) { unsafe { (*parser).pause() } }

/// Resumes the parser.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_resume(parser: *mut Parser) { unsafe { (*parser).resume() } }

/// Completes the current message without consuming more input.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_complete(parser: *mut Parser) {
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).complete();
    }
  }
}

/// Marks the parser as finished. Any new data received via `milo_parse` will
/// put the parser in the error state.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_finish(parser: *mut Parser) {
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).finish();
    }
  }
}

/// Marks the parsing a failed, setting a error code and and error message.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
/// If `description.ptr` is non-null and `description.len` is nonzero, the bytes
/// must be initialized, readable in one allocation, and unchanged for the
/// duration of the call. The length must not exceed `isize::MAX` and the bytes
/// must not overlap the parser or its owned buffers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_fail(parser: *mut Parser, code: u8, description: CStringWithLength) {
  // SAFETY: The caller provides a live parser.
  if unsafe { (*parser).parsing } {
    return;
  }

  let description = if description.len == 0 {
    ""
  } else if description.ptr.is_null() {
    "Invalid error description pointer"
  } else {
    // SAFETY: The caller guarantees a valid, separate description buffer.
    let bytes = unsafe { slice::from_raw_parts(description.ptr, description.len) };
    str::from_utf8(bytes).unwrap_or("Invalid UTF-8 error description")
  };
  // SAFETY: The caller guarantees exclusive access to a live parser.
  unsafe { (*parser).fail(code, description) };
}

/// Returns the current parser's state as string.
///
/// The returned value must be freed using `free_string`.
///
/// # Safety
/// `parser` must point to a live parser.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_state_string(parser: *mut Parser) -> CStringWithLength {
  unsafe { (*parser).state_str().into() }
}

/// Returns a parser method as string.
///
/// The returned value must be freed using `free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn milo_method_to_string(method: u8) -> CStringWithLength {
  Methods::try_from(method)
    .map_or("UNKNOWN", |method| method.as_str())
    .into()
}

/// Returns a parser error as string.
///
/// The returned value must be freed using `free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn milo_error_to_string(error: u8) -> CStringWithLength {
  Errors::try_from(error).map_or("UNKNOWN", |error| error.as_str()).into()
}

/// Returns a parser callback as string.
///
/// The returned value must be freed using `free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn milo_callback_to_string(callback: u8) -> CStringWithLength {
  Callbacks::try_from(callback)
    .map_or("UNKNOWN", |callback| callback.as_str())
    .into()
}

/// Returns a parser state as string.
///
/// The returned value must be freed using `free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn milo_state_to_string(state: u8) -> CStringWithLength {
  States::try_from(state).map_or("UNKNOWN", |state| state.as_str()).into()
}

/// Returns a parser event as string.
///
/// The returned value must be freed using `free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn milo_event_to_string(event: u8) -> CStringWithLength {
  Events::try_from(event).map_or("UNKNOWN", |event| event.as_str()).into()
}

/// Returns the current parser's error state as string.
///
/// The returned value must be freed using `free_string`.
///
/// # Safety
/// `parser` must point to a live parser.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_error_code_string(parser: *mut Parser) -> CStringWithLength {
  unsafe { (*parser).error_code_str().into() }
}

/// Returns the current parser's error descrition.
///
/// The returned value must be freed using `free_string`.
///
/// # Safety
/// `parser` must point to a live parser.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn milo_error_description_string(parser: *mut Parser) -> CStringWithLength {
  unsafe { (*parser).error_description_str().into() }
}
