use proc_macro::TokenStream;
use quote::{format_ident, quote};

use crate::structs::IdentifierWithExpr;

// Handles a callback.
pub fn callback(definition: &IdentifierWithExpr) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;

  if let Some(length) = &definition.expr {
    quote! { (self.callbacks.#callback)(self, self.position, #length); }
  } else {
    quote! { (self.callbacks.#callback)(self, 0, 0); }
  }
}

/// Generates all parser callbacks.
pub fn generate_callbacks(callbacks: &Vec<String>) -> TokenStream {
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
