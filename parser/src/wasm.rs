use core::ffi::{c_uchar, c_void};
use std::slice;

use milo_macros::wasm_getter;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

use crate::Parser;

#[cfg(debug_assertions)]
#[wasm_bindgen(start)]
fn init_error_handler() { std::panic::set_hook(Box::new(console_error_panic_hook::hook)); }

#[cfg(debug_assertions)]
pub fn debug(message: String) { crate::logger(((message.as_ptr() as u64) << 32) + message.len() as u64) }

#[wasm_bindgen]
pub fn alloc(len: usize) -> *mut c_void {
  let buffer = Vec::with_capacity(len);
  let (ptr, _, _) = { buffer.into_raw_parts() };
  ptr as *mut c_void
}

#[wasm_bindgen]
pub fn dealloc(ptr: *mut c_void, len: usize) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Vec::from_raw_parts(ptr, len, len);
  }
}

/// Creates a new parser.
#[wasm_bindgen]
pub fn create() -> *mut c_void {
  let ptr = Box::into_raw(Box::new(Parser::new())) as *mut c_void;

  // Recreate the parser from the box to assign the reference to itself
  let mut parser = unsafe { Box::from_raw(ptr as *mut Parser) };
  parser.ptr = ptr;
  Box::into_raw(parser);

  ptr
}

/// Destroys a parser.
#[wasm_bindgen]
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
#[wasm_bindgen]
pub fn reset(parser: *mut c_void, keep_parsed: bool) { unsafe { (*(parser as *mut Parser)).reset(keep_parsed) } }

/// Clears all values in the parser.
///
/// Persisted fields, unconsumed data and the position are not cleared.
#[wasm_bindgen]
pub fn clear(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).clear() } }

// Parses a slice of characters. It returns the number of consumed characters.
#[wasm_bindgen]
pub fn parse(parser: *mut c_void, data: *const c_uchar, limit: usize) -> usize {
  unsafe { (*(parser as *mut Parser)).parse(data, limit) }
}

/// Pauses the parser. It will have to be resumed via `resume`.
#[wasm_bindgen]
pub fn pause(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).pause() } }

/// Resumes the parser.
#[wasm_bindgen]
pub fn resume(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).resume() } }

/// Marks the parser as finished. Any new data received via `parse` will
/// put the parser in the error state.
#[wasm_bindgen]
pub fn finish(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).finish() } }

/// Marks the parser as failed.
#[wasm_bindgen]
pub fn fail(parser: *mut c_void, code: usize, description_ptr: *const c_uchar, description_len: usize) {
  unsafe {
    (*(parser as *mut Parser)).fail(
      code,
      std::str::from_utf8_unchecked(slice::from_raw_parts(description_ptr, description_len)),
    );
  }
}

// Getters
// User writable
wasm_getter!(mode, getMode, usize);
wasm_getter!(paused, isPaused, bool);
wasm_getter!(manage_unconsumed, manageUnconsumed, bool);
wasm_getter!(continue_without_data, continueWithoutData, bool);
wasm_getter!(is_connect, isConnect, bool);
wasm_getter!(skip_body, skipBody, bool);
// Generic state
wasm_getter!(state, getState, usize);
wasm_getter!(position, getPosition, usize);
wasm_getter!(parsed, getParsed, u64);
wasm_getter!(error_code, getErrorCode, usize);
// Current message flags
wasm_getter!(message_type, getMessageType, usize);
wasm_getter!(method, getMethod, usize);
wasm_getter!(status, getStatus, usize);
wasm_getter!(version_major, getVersionMajor, usize);
wasm_getter!(version_minor, getVersionMinor, usize);
wasm_getter!(connection, getConnection, usize);
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
#[wasm_bindgen(js_name=getErrorDescriptionRaw)]
pub fn get_error_description(parser: *mut c_void) -> u64 {
  let parser = unsafe { &(*(parser as *const Parser)) };

  let ptr = parser.error_description as u64;
  let len = parser.error_description_len as u64;

  (ptr << 32) + len
}

#[wasm_bindgen(js_name=setMode)]
pub fn set_mode(parser: *mut c_void, value: usize) {
  unsafe {
    (*(parser as *mut Parser)).mode = value;
  }
}

#[wasm_bindgen(js_name=setManageUnconsumed)]
pub fn set_manage_unconsumed(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).manage_unconsumed = value;
  }
}

#[wasm_bindgen(js_name=setContinueWithoutData)]
pub fn set_continue_without_data(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).continue_without_data = value;
  }
}

#[wasm_bindgen(js_name=setSkipBody)]
pub fn set_skip_body(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).skip_body = value;
  }
}

#[wasm_bindgen(js_name=setIsConnect)]
pub fn set_is_connect(parser: *mut c_void, value: bool) {
  unsafe {
    (*(parser as *mut Parser)).is_connect = value;
  }
}
