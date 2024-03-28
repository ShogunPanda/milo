use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::{definitions::CALLBACKS, parsing::IdentifiersWithExpr};

// Handles a callback.
pub fn callback_native(definition: &IdentifiersWithExpr, target: &Ident) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;
  let target = format_ident!("{}", target);

  let invocation = if let Some(length) = &definition.expr {
    quote! { (#target.callbacks.#callback)(#target, #target.position, #length); }
  } else {
    quote! { (#target.callbacks.#callback)(#target, 0, 0); }
  };

  quote! { #invocation; }
}

/// Generates all parser callbacks.
pub fn generate_callbacks_native() -> TokenStream {
  let callbacks: Vec<_> = unsafe {
    CALLBACKS
      .get()
      .unwrap()
      .iter()
      .map(|x| format_ident!("{}", x))
      .collect()
  };

  TokenStream::from(quote! {
    #[cfg(not(target_family = "wasm"))]
    fn noop_internal(_parser: &mut Parser, _data: usize, _len: usize) {}

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
