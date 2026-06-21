use proc_macro::TokenStream;
use quote::{format_ident, quote};

use crate::structs::CallbackRequest;

pub fn callback(definition: &CallbackRequest) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;

  if callback.to_string() == "on_headers" {
    return quote! {
      unsafe {
        on_headers(
          self.ptr,
          self.position,
          if self.is_request { self.method as u32 } else { self.status as u32 },
          !self.has_connection_close,
          self.has_upgrade && self.has_connection_upgrade,
          self.has_trailers,
          if self.has_content_length { 0 } else if self.has_chunked_transfer_encoding { 1 } else { 2 },
          if self.has_content_length { self.content_length as f64 } else { 0.0 },
        )
      };
    };
  }

  if let Some(offset) = &definition.offset
    && let Some(length) = &definition.length
  {
    quote! { unsafe { #callback(self.ptr, self.position + #offset, #length) }; }
  } else {
    quote! { unsafe { #callback(self.ptr, self.position, 0) }; }
  }
}

/// Generates all parser callbacks.
pub fn generate_callbacks(callbacks: &[String]) -> TokenStream {
  let callbacks: Vec<_> = callbacks
    .iter()
    .filter(|x| x.as_str() != "on_headers")
    .map(|x| format_ident!("{}", x))
    .collect();

  TokenStream::from(quote! {
    #[cfg(target_family = "wasm")]
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
      #(fn #callbacks(parser: *mut c_void, _at: usize, _len: usize);)*
      fn on_headers(
        parser: *mut c_void,
        at: usize,
        method_or_status: u32,
        should_keep_alive: bool,
        should_upgrade: bool,
        has_trailers: bool,
        body_kind: u8,
        content_length: f64,
      );

      #[cfg(any(debug_assertions, feature = "debug"))]
      fn logger(message: u64);
    }

    #[cfg(all(any(debug_assertions, feature = "debug"), target_family = "wasm"))]
    #[unsafe(no_mangle)]
    pub fn __start() {
      std::panic::set_hook(Box::new(|panic_info| {
        debug(format!("WebAssembly panicked: {:#?}", panic_info));
      }));
    }
  })
}
