use proc_macro::TokenStream;
use quote::{format_ident, quote};

use crate::structs::CallbackRequest;

pub fn callback(definition: &CallbackRequest) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;

  if let Some(offset) = &definition.offset
    && let Some(length) = &definition.length
  {
    quote! {
      (self.callbacks.#callback)(self, self.position + #offset, #length);
    }
  } else {
    quote! { (self.callbacks.#callback)(self, self.position, 0); }
  }
}

/// Generates all parser callbacks.
pub fn generate_callbacks(callbacks: &[String]) -> TokenStream {
  let callbacks: Vec<_> = callbacks.iter().map(|x| format_ident!("{}", x)).collect();

  TokenStream::from(quote! {
    #[cfg(not(target_family = "wasm"))]
    fn noop_internal(_parser: &mut Parser, _at: usize, _len: usize) {}

    #[cfg(not(target_family = "wasm"))]
    #[repr(C)]
    #[derive(Clone, Debug)]
    pub struct ParserCallbacks {
      #( pub #callbacks: Callback),*
    }

    #[cfg(not(target_family = "wasm"))]
    impl ParserCallbacks {
      fn new() -> ParserCallbacks {
        ParserCallbacks {
          #( #callbacks: noop_internal ),*
        }
      }
    }
  })
}
