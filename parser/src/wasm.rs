//! Raw WebAssembly exports. A parser pointer must refer to a live, aligned
//! parser returned by `create`. Mutating it requires exclusive access for the
//! duration of the call; read-only operations must not overlap mutation.

use core::ffi::{c_uchar, c_void};
use std::slice;

use crate::Parser;

#[cfg(any(debug_assertions, feature = "debug"))]
pub fn debug(message: String) { unsafe { crate::logger(((message.as_ptr() as u64) << 32) + message.len() as u64) } }

/// Returns if debug informations are available in this build.
#[unsafe(no_mangle)]
pub fn milo_has_debug() -> bool { cfg!(any(debug_assertions, feature = "debug")) }

#[unsafe(no_mangle)]
pub fn alloc(len: usize) -> *mut c_void {
  let buffer = Vec::with_capacity(len);
  let (ptr, _, _) = { buffer.into_raw_parts() };
  ptr as *mut c_void
}

/// # Safety
/// `ptr` must be null or a live pointer from `alloc`, with the same `len` used
/// to allocate it. The allocation must not be used after this call.
#[unsafe(no_mangle)]
pub unsafe fn dealloc(ptr: *mut c_void, len: usize) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Vec::from_raw_parts(ptr, len, len);
  }
}

/// Creates a new parser.
#[unsafe(no_mangle)]
pub fn create() -> *mut c_void {
  let ptr = Box::into_raw(Box::new(Parser::new())) as *mut c_void;

  // Recreate the parser from the box to assign the reference to itself
  let mut parser = unsafe { Box::from_raw(ptr as *mut Parser) };
  parser.ptr = ptr;
  let _ = Box::into_raw(parser);

  ptr
}

/// Destroys a parser. During callback replay this is a no-op.
///
/// Returns whether destruction occurred so the JS Simple API only clears collected
/// spans and overflow state after the parser has been destroyed.
///
/// # Safety
/// `parser` must be null or a live pointer returned by `create`. The caller
/// must have exclusive access except during its callback replay.
/// Do not use the parser after successful destruction.
#[unsafe(no_mangle)]
pub unsafe fn destroy(parser: *mut c_void) -> bool {
  let parser = parser.cast::<Parser>();
  // SAFETY: A non-null parser must be live by contract.
  if parser.is_null() || unsafe { (*parser).parsing } {
    return false;
  }

  // SAFETY: The parser is owned by the caller and is not in replay.
  unsafe {
    let _ = Box::from_raw(parser);
  }
  true
}

/// Resets a parser. The second parameters specifies if to also reset the
/// parsed counter.
///
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn reset(parser: *mut c_void, keep_parsed: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).reset(keep_parsed);
    }
  }
}

/// Clears all values in the parser.
///
/// Persisted fields, unconsumed data and the position are not cleared.
///
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn clear(parser: *mut c_void) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).clear();
    }
  }
}

/// Parses a byte range. It returns the number of consumed bytes.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible for the call.
/// If `limit` is nonzero, `data` must point to `limit` initialized bytes in one
/// allocation, with `limit <= isize::MAX`. Those bytes must remain valid and
/// unchanged and must not overlap the parser or its owned buffers. If `limit`
/// is zero, `data` may be null.
#[unsafe(no_mangle)]
pub unsafe fn parse(parser: *mut c_void, data: *const c_uchar, limit: usize) -> usize {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller provides a live parser.
  if unsafe { (*parser).parsing } {
    return 0;
  }

  let input = if limit == 0 {
    &[]
  } else {
    // SAFETY: The caller guarantees a valid, separate input buffer.
    unsafe { slice::from_raw_parts(data, limit) }
  };
  // SAFETY: The caller guarantees exclusive access to a live parser.
  unsafe { (*parser).parse_body(input) }
}

/// Pauses the parser. It will have to be resumed via `resume`.
///
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn pause(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).pause() } }

/// Resumes the parser.
///
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn resume(parser: *mut c_void) { unsafe { (*(parser as *mut Parser)).resume() } }

/// Completes the current message without consuming more input.
///
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn complete(parser: *mut c_void) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).complete();
    }
  }
}

/// Marks the parser as finished. Any new data received via `parse` will
/// put the parser in the error state.
///
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn finish(parser: *mut c_void) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller provides a live parser. Borrow only outside replay.
  unsafe {
    if !(*parser).parsing {
      (*parser).finish();
    }
  }
}

/// Marks the parser as failed.
///
/// # Safety
/// The parser must be live, aligned, and exclusively accessible for the call.
/// If `description_ptr` is non-null and `description_len` is nonzero, the bytes
/// must be initialized, readable in one allocation, and unchanged for the
/// duration of the call. The length must not exceed `isize::MAX` and the bytes
/// must not overlap the parser or its owned buffers.
#[unsafe(no_mangle)]
pub unsafe fn fail(parser: *mut c_void, code: u8, description_ptr: *const c_uchar, description_len: usize) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller provides a live parser.
  if unsafe { (*parser).parsing } {
    return;
  }

  let description = if description_len == 0 {
    ""
  } else if description_ptr.is_null() {
    "Invalid error description pointer"
  } else {
    // SAFETY: The caller guarantees a valid, separate description buffer.
    let bytes = unsafe { slice::from_raw_parts(description_ptr, description_len) };
    std::str::from_utf8(bytes).unwrap_or("Invalid UTF-8 error description")
  };
  // SAFETY: The caller guarantees exclusive access to a live parser.
  unsafe {
    (*parser).fail(code, description);
  }
}

// Generates a getter.
// pub fn wasm_getter(input: TokenStream) -> TokenStream {
// let snake_matcher = Regex::new(r"([A-Z])").unwrap();
//
// let definition = parse_macro_input!(input as Property);
// let property = definition.property;
// let fn_name = format_ident!(
// "{}",
// snake_matcher.replace_all(&definition.getter.to_string().as_str(), |captures:
// &Captures| { format!("_{}", captures[1].to_lowercase())
// })
// );
//
// let return_type = definition.r#type;
//
// TokenStream::from(quote! {
// Gets the parser #property.
// #[unsafe(no_mangle)]
// pub fn #fn_name(parser: *const c_void) -> #return_type { unsafe { (*(parser
// as *const Parser)).#property } } })
// }

// Getters
// Get the parser autodetect property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn is_autodetect(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).autodetect } }

// Get the parser is_request property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn is_request(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).is_request } }

// Get the parser debug property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn is_debug(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).debug } }

// Get the parser paused property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn is_paused(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).paused } }

// Get the parser manage_unconsumed property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn should_manage_unconsumed(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).manage_unconsumed }
}

// Get the parser suspend_after_headers property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn should_suspend_after_headers(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).suspend_after_headers }
}

/// # Safety
/// The parser must be live and aligned.
#[unsafe(no_mangle)]
pub unsafe fn should_require_host(parser: *const c_void) -> bool {
  // SAFETY: Required by this function's contract.
  unsafe { (*parser.cast::<Parser>()).require_host }
}

// Get the parser max_start_line_length property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_max_start_line_length(parser: *const c_void) -> usize {
  unsafe { (*(parser as *const Parser)).max_start_line_length }
}

// Get the parser max_field_length property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_max_field_length(parser: *const c_void) -> usize {
  unsafe { (*(parser as *const Parser)).max_field_length }
}

/// # Safety
/// The parser must be live and aligned.
#[unsafe(no_mangle)]
pub unsafe fn get_max_fields_bytes(parser: *const c_void) -> usize {
  // SAFETY: Required by this function's contract.
  unsafe { (*parser.cast::<Parser>()).max_fields_bytes }
}

/// # Safety
/// The parser must be live and aligned.
#[unsafe(no_mangle)]
pub unsafe fn get_max_fields_count(parser: *const c_void) -> usize {
  // SAFETY: Required by this function's contract.
  unsafe { (*parser.cast::<Parser>()).max_fields_count }
}

// Get the parser max_body_payload property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_max_body_payload(parser: *const c_void) -> u64 {
  unsafe { (*(parser as *const Parser)).max_body_payload }
}

// Get the parser continue_without_data property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn should_continue_without_data(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).continue_without_data }
}

// Get the parser is_connect property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn is_connect(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).is_connect } }

// Get the parser skip_body property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn should_skip_body(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).skip_body } }

// Get the parser state property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_state(parser: *const c_void) -> u8 { unsafe { (*(parser as *const Parser)).state } }

// Get the parser position property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_position(parser: *const c_void) -> usize { unsafe { (*(parser as *const Parser)).position } }

// Get the parser parsed property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_parsed(parser: *const c_void) -> u64 { unsafe { (*(parser as *const Parser)).parsed } }

// Get the parser error_code property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_error_code(parser: *const c_void) -> u8 { unsafe { (*(parser as *const Parser)).error_code } }

// Get the parser method property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_method(parser: *const c_void) -> u8 { unsafe { (*(parser as *const Parser)).method } }

// Get the parser status property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_status(parser: *const c_void) -> u32 { unsafe { (*(parser as *const Parser)).status } }

// Get the parser has_connection_close property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn has_connection_close(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).has_connection_close }
}

// Get the parser has_connection_upgrade property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn has_connection_upgrade(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).has_connection_upgrade }
}

// Get the parser content_length property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_content_length(parser: *const c_void) -> u64 {
  unsafe { (*(parser as *const Parser)).content_length }
}

// Get the parser chunk_size property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_chunk_size(parser: *const c_void) -> u64 { unsafe { (*(parser as *const Parser)).chunk_size } }

// Get the parser remaining_content_length property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_remaining_content_length(parser: *const c_void) -> u64 {
  unsafe { (*(parser as *const Parser)).remaining_content_length }
}

// Get the parser remaining_chunk_size property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_remaining_chunk_size(parser: *const c_void) -> u64 {
  unsafe { (*(parser as *const Parser)).remaining_chunk_size }
}

// Get the parser has_content_length property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn has_content_length(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).has_content_length }
}

// Get the parser has_transfer_encoding property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn has_transfer_encoding(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).has_transfer_encoding }
}

// Get the parser has_chunked_transfer_encoding property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn has_chunked_transfer_encoding(parser: *const c_void) -> bool {
  unsafe { (*(parser as *const Parser)).has_chunked_transfer_encoding }
}

// Get the parser has_upgrade property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn has_upgrade(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).has_upgrade } }

// Get the parser has_trailers property.
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn has_trailers(parser: *const c_void) -> bool { unsafe { (*(parser as *const Parser)).has_trailers } }

/// Gets the parser callback error description, if any. This is meant for
/// internal use.
///
/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn get_error_description_raw(parser: *mut c_void) -> u64 {
  let parser = unsafe { &(*(parser as *const Parser)) };

  let ptr = parser.error_description.as_ptr() as u64;
  let len = parser.error_description_len as u64;

  (ptr << 32) + len
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_should_autodetect(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).autodetect = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_is_request(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).is_request = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_should_manage_unconsumed(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).manage_unconsumed = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_should_suspend_after_headers(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).suspend_after_headers = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_should_require_host(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: Required by this function's contract.
  unsafe {
    if !(*parser).parsing {
      (*parser).require_host = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_max_start_line_length(parser: *mut c_void, value: usize) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).max_start_line_length = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_max_field_length(parser: *mut c_void, value: usize) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).max_field_length = value;
    }
  }
}

/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe fn set_max_fields_bytes(parser: *mut c_void, value: usize) {
  let parser = parser.cast::<Parser>();
  // SAFETY: Required by this function's contract.
  unsafe {
    if !(*parser).parsing {
      (*parser).max_fields_bytes = value;
    }
  }
}

/// # Safety
/// The parser must be live, aligned, and exclusively accessible.
#[unsafe(no_mangle)]
pub unsafe fn set_max_fields_count(parser: *mut c_void, value: usize) {
  let parser = parser.cast::<Parser>();
  // SAFETY: Required by this function's contract.
  unsafe {
    if !(*parser).parsing {
      (*parser).max_fields_count = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_max_body_payload(parser: *mut c_void, value: u64) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).max_body_payload = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_should_continue_without_data(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).continue_without_data = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_should_skip_body(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).skip_body = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_is_connect(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).is_connect = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_debug(parser: *mut c_void, value: bool) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).debug = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_active_callbacks(parser: *mut c_void, value: u64) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).active_callbacks = value;
    }
  }
}

/// # Safety
/// Every raw pointer must satisfy the exported WebAssembly ABI contract.
#[unsafe(no_mangle)]
pub unsafe fn set_active_events(parser: *mut c_void, value: u64) {
  let parser = parser.cast::<Parser>();
  // SAFETY: The caller supplies a live parser on the current thread.
  unsafe {
    if !(*parser).parsing {
      (*parser).active_events = value;
    }
  }
}
