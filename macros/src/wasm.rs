use proc_macro::TokenStream;
use quote::{format_ident, quote};

use crate::structs::CallbackRequest;

pub fn callback(definition: &CallbackRequest) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;

  if let Some(offset) = &definition.offset
    && let Some(length) = &definition.length
  {
    quote! { unsafe { #callback(self.ptr, self.position + #offset, #length) }; }
  } else {
    quote! { unsafe { #callback(self.ptr, self.position, 0) }; }
  }
}

/// Generates all parser callbacks.
pub fn generate_callbacks(callbacks: &Vec<String>) -> TokenStream {
  let callbacks: Vec<_> = callbacks.iter().map(|x| format_ident!("{}", x)).collect();

  TokenStream::from(quote! {
    #[cfg(target_family = "wasm")]
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
      #(fn #callbacks(parser: *mut c_void, _at: usize, _len: usize);)*

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
