use wasm_bindgen::prelude::*;

use crate::*;

#[wasm_bindgen]
extern "C" {
  // Use `js_namespace` here to bind `console.log(..)` instead of just
  // `log(..)`
  #[wasm_bindgen(js_namespace = console)]
  fn log(s: &str);
}

// This impl only contains the parse_wasm method which is exported to WASM
#[cfg(target_family = "wasm")]
#[wasm_bindgen]
impl Parser {
  /// Creates a new parser.
  #[wasm_bindgen(constructor)]
  pub fn new_wasm(id: Option<u8>) -> Parser {
    // TODO@PI: Move this to a static file
    #[cfg(debug_assertions)]
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    let parser = Parser::new();
    parser.id.set(id.unwrap_or(0));
    parser
  }

  #[wasm_bindgen]
  pub fn destroy(&self) {
    unsafe {
      let _ = Vec::from_raw_parts(self.offsets.get(), MAX_OFFSETS_COUNT, MAX_OFFSETS_COUNT);
    }
  }

  #[wasm_bindgen]
  pub fn parse(&self, data: *const c_uchar, limit: usize) -> Result<usize, JsValue> {
    // If the parser is paused, this is a no-op
    if self.paused.get() {
      return Ok(0);
    }

    let data = unsafe { from_raw_parts(data, limit) };

    parse!();

    Ok(consumed)
  }

  // TODO@PI: Here and in Rust - Can you move all these flags to a contigous
  // memory so that in WebAssembly you can access it directly?

  #[wasm_bindgen(getter = state)]
  pub fn get_state(&self) -> u8 { self.state.get() }

  #[wasm_bindgen(getter = position)]
  pub fn get_position(&self) -> usize { self.position.get() }

  #[wasm_bindgen(getter = parsed)]
  pub fn get_parsed(&self) -> u64 { self.parsed.get() }

  #[wasm_bindgen(getter = paused)]
  pub fn get_paused(&self) -> bool { self.paused.get() }

  #[wasm_bindgen(getter = errorCode)]
  pub fn get_error_code(&self) -> u8 { self.error_code.get() }

  #[wasm_bindgen(getter = errorDescription)]
  pub fn get_error_description(&self) -> JsValue {
    unsafe {
      str::from_utf8_unchecked(slice::from_raw_parts(
        self.error_description.get(),
        self.error_description_len.get(),
      ))
      .into()
    }
  }

  #[wasm_bindgen(getter = callbackError)]
  pub fn get_callback_error(&self) -> JsValue { self.callback_error.borrow().clone() }

  #[wasm_bindgen(getter = id)]
  pub fn get_id(&self) -> u8 { self.id.get() }

  #[wasm_bindgen(setter = id)]
  pub fn set_id(&self, value: u8) { self.id.set(value); }

  #[wasm_bindgen(getter = mode)]
  pub fn get_mode(&self) -> u8 { self.mode.get() }

  #[wasm_bindgen(setter = mode)]
  pub fn set_mode(&self, value: u8) { self.mode.set(value); }

  #[wasm_bindgen(getter = manageUnconsumed)]
  pub fn get_manage_unconsumed(&self) -> bool { self.manage_unconsumed.get() }

  #[wasm_bindgen(setter = manageUnconsumed)]
  pub fn set_manage_unconsumed(&self, value: bool) { self.manage_unconsumed.set(value); }

  #[wasm_bindgen(getter = continueWithoutData)]
  pub fn get_continue_without_data(&self) -> bool { self.continue_without_data.get() }

  #[wasm_bindgen(getter = messageType)]
  pub fn get_message_type(&self) -> u8 { self.message_type.get() }

  #[wasm_bindgen(getter = isConnect)]
  pub fn get_is_connect(&self) -> bool { self.is_connect.get() }

  #[wasm_bindgen(setter = isConnect)]
  pub fn set_is_connect(&self, value: bool) { self.is_connect.set(value); }

  #[wasm_bindgen(getter = method)]
  pub fn get_method(&self) -> u8 { self.method.get() }

  #[wasm_bindgen(getter = status)]
  pub fn get_status(&self) -> usize { self.status.get() }

  #[wasm_bindgen(getter = versionMajor)]
  pub fn get_version_major(&self) -> u8 { self.version_major.get() }

  #[wasm_bindgen(getter = versionMinor)]
  pub fn get_version_minor(&self) -> u8 { self.version_minor.get() }

  #[wasm_bindgen(getter = connection)]
  pub fn get_connection(&self) -> u8 { self.connection.get() }

  #[wasm_bindgen(getter = hasContentLength)]
  pub fn get_has_content_length(&self) -> bool { self.has_content_length.get() }

  #[wasm_bindgen(getter = hasChunkedTransferEncoding)]
  pub fn get_has_chunked_transfer_encoding(&self) -> bool { self.has_chunked_transfer_encoding.get() }

  #[wasm_bindgen(getter = hasUpgrade)]
  pub fn get_has_upgrade(&self) -> bool { self.has_upgrade.get() }

  #[wasm_bindgen(getter = hasTrailers)]
  pub fn get_has_trailers(&self) -> bool { self.has_trailers.get() }

  #[wasm_bindgen(getter = contentLength)]
  pub fn get_content_length(&self) -> u64 { self.content_length.get() }

  #[wasm_bindgen(getter = chunkSize)]
  pub fn get_chunk_size(&self) -> u64 { self.chunk_size.get() }

  #[wasm_bindgen(getter = remainingContentLength)]
  pub fn get_remaining_content_length(&self) -> u64 { self.remaining_content_length.get() }

  #[wasm_bindgen(getter = remainingChunkSize)]
  pub fn get_remaining_chunk_size(&self) -> u64 { self.remaining_chunk_size.get() }

  #[wasm_bindgen(getter = unconsumed)]
  pub fn get_unconsumed_len(&self) -> usize { self.unconsumed_len.get() }

  #[wasm_bindgen(getter = skipBody)]
  pub fn get_skip_body(&self) -> bool { self.skip_body.get() }

  #[wasm_bindgen(setter = skipBody)]
  pub fn set_skip_body(&self, value: bool) { self.skip_body.set(value); }

  #[wasm_bindgen(getter = offsets)]
  pub fn get_offsets(&self) -> js_sys::Uint32Array {
    unsafe { js_sys::Uint32Array::view_mut_raw(self.offsets.get() as *mut _, MAX_OFFSETS_COUNT) }
  }
}

#[wasm_bindgen(js_name = alloc)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
  let buffer = vec![0; len];
  let (ptr, _, _) = { buffer.into_raw_parts() };
  ptr
}

#[wasm_bindgen(js_name = free)]
pub extern "C" fn free(ptr: *mut u8, len: usize) {
  if ptr.is_null() {
    return;
  }

  unsafe {
    let _ = Vec::from_raw_parts(ptr, len, len);
  }
}
