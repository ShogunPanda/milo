use core::ffi::{c_uchar, c_void};
use std::slice;

use milo_macros::wasm_getter;

use crate::Parser;

#[cfg(debug_assertions)]
pub fn debug(message: String) { unsafe { crate::logger(((message.as_ptr() as u64) << 32) + message.len() as u64) } }

#[no_mangle]
pub fn alloc(len: usize) -> *mut c_void {
  let buffer = Vec::with_capacity(len);
  let (ptr, _, _) = { buffer.into_raw_parts() };
  ptr as *mut c_void
}

#[no_mangle]
pub fn dealloc(ptr: *mut c_void, len: usize) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Vec::from_raw_parts(ptr, len, len);
  }
}

/// Creates a new parser.
#[no_mangle]
pub fn create() -> *mut c_void {
  let ptr = Box::into_raw(Box::new(Parser::new())) as *mut c_void;

  // Recreate the parser from the box to assign the reference to itself
  let mut parser = unsafe { Box::from_raw(ptr as *mut Parser) };
  parser.ptr = ptr;
  Box::into_raw(parser);

  ptr
}

/// Destroys a parser.
#[no_mangle]
pub fn destroy(parser: *mut c_void) {
  if parser.is_null() {
    return;
  }

  unsafe {
    let _ = Box::from_raw(parser as *mut Parser);
  }
}

/// Resets a parser. The second parameters specifies if to also reset the
/// parsed counter.
#[no_mangle]
pub fn reset(parser: *mut c_void, keep_parsed: bool) { unsafe { (*(parser as *mut Parser)).reset(keep_parsed) } }

/// Clears all values in the parser.
///
/// Persisted fields, unconsumed data and the position are not cleared.
#[no_mangle]
pub fn clear(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).clear() } }

// Parses a slice of characters. It returns the number of consumed characters.
#[no_mangle]
pub fn parse(parser: *mut c_void, data: *const c_uchar, limit: usize) -> usize {
  unsafe { (*(parser as *mut Parser)).parse(data, limit) }
}

/// Pauses the parser. It will have to be resumed via `resume`.
#[no_mangle]
pub fn pause(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).pause() } }

/// Resumes the parser.
#[no_mangle]
pub fn resume(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).resume() } }

/// Marks the parser as finished. Any new data received via `parse` will
/// put the parser in the error state.
#[no_mangle]
pub fn finish(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).finish() } }

/// Marks the parser as failed.
#[no_mangle]
pub fn fail(parser: *mut c_void, code: u8, description_ptr: *const c_uchar, description_len: usize) {
  unsafe {
    (*(parser as *mut Parser)).fail(
      code,
      std::str::from_utf8_unchecked(slice::from_raw_parts(description_ptr, description_len)),
    );
  }
}

// Getters
// User writable
wasm_getter!(mode, getMode, u8);
wasm_getter!(paused, isPaused, bool);
wasm_getter!(manage_unconsumed, manageUnconsumed, bool);
wasm_getter!(continue_without_data, continueWithoutData, bool);
wasm_getter!(is_connect, isConnect, bool);
wasm_getter!(skip_body, skipBody, bool);
// Generic state
wasm_getter!(state, getState, u8);
wasm_getter!(position, getPosition, usize);
wasm_getter!(parsed, getParsed, u64);
wasm_getter!(error_code, getErrorCode, u8);
// Current message flags
wasm_getter!(message_type, getMessageType, u8);
wasm_getter!(method, getMethod, u8);
wasm_getter!(status, getStatus, u32);
wasm_getter!(version_major, getVersionMajor, u8);
wasm_getter!(version_minor, getVersionMinor, u8);
wasm_getter!(connection, getConnection, u8);
wasm_getter!(content_length, getContentLength, u64);
wasm_getter!(chunk_size, getChunkSize, u64);
wasm_getter!(remaining_content_length, getRemainingContentLength, u64);
wasm_getter!(remaining_chunk_size, getRemainingChunkSize, u64);
wasm_getter!(has_content_length, hasContentLength, bool);
wasm_getter!(has_chunked_transfer_encoding, hasChunkedTransferEncoding, bool);
wasm_getter!(has_upgrade, hasUpgrade, bool);
wasm_getter!(has_trailers, hasTrailers, bool);

/// Gets the parser callback error description, if any. This is meant for
/// internal use.
#[no_mangle]
pub fn get_error_description_raw(parser: *mut c_void) -> u64 {
  let parser = unsafe { &(*(parser as *const Parser)) };

  let ptr = parser.error_description as u64;
  let len = parser.error_description_len as u64;

  (ptr << 32) + len
}

#[no_mangle]
pub fn set_mode(parser: *mut c_void, value: u8) {
  unsafe {
    (*(parser as *mut Parser)).mode = value;
  }
}

#[no_mangle]
pub fn set_manage_unconsumed(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).manage_unconsumed = value;
  }
}

#[no_mangle]
pub fn set_continue_without_data(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).continue_without_data = value;
  }
}

#[no_mangle]
pub fn set_skip_body(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).skip_body = value;
  }
}

#[no_mangle]
pub fn set_is_connect(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).is_connect = value;
  }
}
