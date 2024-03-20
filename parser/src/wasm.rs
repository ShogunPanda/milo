use core::ffi::{c_uchar, c_void};
use std::slice;

use milo_macros::wasm_getter;
use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

use crate::{Parser, MAX_OFFSETS_COUNT};

#[cfg(debug_assertions)]
#[wasm_bindgen(start)]
fn init_error_handler() { std::panic::set_hook(Box::new(console_error_panic_hook::hook)); }

#[wasm_bindgen(js_name = alloc)]
pub fn alloc(len: usize) -> *mut c_void {
  let buffer = Vec::with_capacity(len);
  let (ptr, _, _) = { buffer.into_raw_parts() };
  ptr as *mut c_void
}

#[wasm_bindgen(js_name = free)]
pub fn free(ptr: *mut c_void, len: usize) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Vec::from_raw_parts(ptr, len, len);
  }
}

/// Creates a new parser.
#[wasm_bindgen]
pub fn create(id: Option<usize>) -> *mut c_void {
  let parser = crate::create(id);
  let ptr = Box::into_raw(Box::new(parser)) as *mut c_void;

  // Temporarily recreate the parser from the box to assign the reference to
  // itself
  let parser = unsafe { Box::from_raw(ptr as *mut Parser) };
  parser.wasm_ptr.set(ptr);
  Box::into_raw(parser);

  ptr
}

/// Destroys a parser.
#[wasm_bindgen]
pub fn destroy(raw: *mut c_void) {
  unsafe {
    let parser = Box::from_raw(raw as *mut Parser);
    let _ = Vec::from_raw_parts(parser.offsets.get(), MAX_OFFSETS_COUNT, MAX_OFFSETS_COUNT);
    Box::into_raw(parser);
  }
}

/// Resets a parser. The second parameters specifies if to also reset the
/// parsed counter.
#[wasm_bindgen]
pub fn reset(raw: *mut c_void, keep_parsed: bool) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  crate::reset(&parser, keep_parsed);
  Box::into_raw(parser);
}

/// Clears all values in the parser.
///
/// Persisted fields, unconsumed data and the position are not cleared.
#[wasm_bindgen]
pub fn clear(raw: *mut c_void) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  crate::clear(&parser);
  Box::into_raw(parser);
}

// Parses a slice of characters. It returns the number of consumed characters.
#[wasm_bindgen]
pub fn parse(raw: *mut c_void, data: *const c_uchar, limit: usize) -> usize {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  let consumed = crate::parse(&parser, data, limit);
  Box::into_raw(parser);
  consumed
}

/// Pauses the parser. It will have to be resumed via `milo_resume`.
#[wasm_bindgen]
pub fn pause(raw: *mut c_void) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  crate::pause(&parser);
  Box::into_raw(parser);
}

/// Resumes the parser.
#[wasm_bindgen]
pub fn resume(raw: *mut c_void) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  crate::resume(&parser);
  Box::into_raw(parser);
}

/// Marks the parser as finished. Any new data received via `parse` will
/// put the parser in the error state.
#[wasm_bindgen]
pub fn finish(raw: *mut c_void) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  crate::finish(&parser);
  Box::into_raw(parser);
}

/// Marks the parser as failed.
#[wasm_bindgen]
pub fn fail(raw: *mut c_void, code: usize, reason: &str) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  crate::fail(&parser, code, reason);
  Box::into_raw(parser);
}

// TODO@PI: Document this (Rust & WASM)
/// Clear the parser offsets.
#[wasm_bindgen(js_name=clearOffsets)]
pub fn clear_offsets(raw: *mut c_void) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  crate::clear_offsets(&parser);
  Box::into_raw(parser);
}

/// Returns the current parser's state as string.
#[wasm_bindgen(js_name=getStateString)]
pub fn state_string(raw: *mut c_void) -> String {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  let value = crate::state_string(&parser).to_string();
  Box::into_raw(parser);

  value
}

/// Returns the current parser's error state as string.
#[wasm_bindgen(js_name=getErrorCodeString)]
pub fn error_code_string(raw: *mut c_void) -> String {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  let value = crate::error_code_string(&parser).to_string();
  Box::into_raw(parser);

  value
}

/// Returns the current parser's error descrition.
#[wasm_bindgen(js_name=getErrorDescriptionString)]
pub fn error_description_string(raw: *mut c_void) -> String {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  let value = crate::error_description_string(&parser).to_string();
  Box::into_raw(parser);

  value
}

// General values
wasm_getter!(state, getState, usize);
wasm_getter!(position, getPosition, usize);
wasm_getter!(error_code, getErrorCode, usize);
wasm_getter!(error_description_len, getErrorDescriptionLen, usize);
wasm_getter!(unconsumed_len, getUnconsumedLen, usize);
wasm_getter!(id, getId, usize);
wasm_getter!(mode, getMode, usize);
wasm_getter!(message_type, getMessageType, usize);
wasm_getter!(method, getMethod, usize);
wasm_getter!(status, getStatus, usize);
wasm_getter!(version_major, getVersionMajor, usize);
wasm_getter!(version_minor, getVersionMinor, usize);
wasm_getter!(connection, getConnection, usize);

// Large values
wasm_getter!(parsed, getParsed, u64);
wasm_getter!(content_length, getContentLength, u64);
wasm_getter!(chunk_size, getChunkSize, u64);
wasm_getter!(remaining_content_length, getRemainingContentLength, u64);
wasm_getter!(remaining_chunk_size, getRemainingChunkSize, u64);

// Flags
wasm_getter!(paused, isPaused, bool);
wasm_getter!(manage_unconsumed, manageUnconsumed, bool);
wasm_getter!(continue_without_data, continueWithoutData, bool);
wasm_getter!(is_connect, isConnect, bool);
wasm_getter!(has_content_length, hasContentLength, bool);
wasm_getter!(has_chunked_transfer_encoding, hasChunkedTransferEncoding, bool);
wasm_getter!(has_upgrade, hasUpgrade, bool);
wasm_getter!(has_trailers, hasTrailers, bool);
wasm_getter!(skip_body, skipBody, bool);

// Pointers
wasm_getter!(offsets, getOffsets, *mut usize);
wasm_getter!(unconsumed, getUnconsumed, *const c_uchar);
wasm_getter!(error_description, getErrorDescription, *const c_uchar);
wasm_getter!(owner, getOwner, *mut c_void);

#[wasm_bindgen(js_name = getCallbackError)]
pub fn get_callback_error(raw: *mut c_void) -> JsValue {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  let value = parser.callback_error.borrow().clone();
  Box::into_raw(parser);

  value
}

#[wasm_bindgen(js_name = setId)]
pub fn set_id(raw: *mut c_void, value: usize) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  parser.id.set(value);
  Box::into_raw(parser);
}

#[wasm_bindgen(js_name = setMode)]
pub fn set_mode(raw: *mut c_void, value: usize) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  parser.mode.set(value);
  Box::into_raw(parser);
}

#[wasm_bindgen(js_name = setManageUnconsumed)]
pub fn set_manage_unconsumed(raw: *mut c_void, value: bool) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  parser.manage_unconsumed.set(value);
  Box::into_raw(parser);
}

#[wasm_bindgen(js_name = setSkipBody)]
pub fn set_skip_body(raw: *mut c_void, value: bool) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  parser.skip_body.set(value);
  Box::into_raw(parser);
}

#[wasm_bindgen(js_name = setIsConnect)]
pub fn set_is_connect(raw: *mut c_void, value: bool) {
  let parser = unsafe { Box::from_raw(raw as *mut Parser) };
  parser.is_connect.set(value);
  Box::into_raw(parser);
}
