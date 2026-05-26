use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, parse_macro_input};

use crate::{
  native,
  structs::{CallbackRequest, FailureRequest},
  wasm,
};

/// Invokes one of the user defined callbacks, eventually attaching some view of
/// the data (via pointer and length).
pub fn callback(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as CallbackRequest);
  let native = native::callback(&definition);
  let wasm = wasm::callback(&definition);

  let bitmask = format_ident!("CALLBACK_ACTIVE_{}", definition.identifier.to_string().to_uppercase());

  TokenStream::from(quote! {
    if self.active_callbacks & #bitmask != 0 {
      #[cfg(not(target_family = "wasm"))]
      #native

      #[cfg(target_family = "wasm")]
      #wasm
    }
  })
}

// Marks a certain number of characters as used.
pub fn advance(input: TokenStream) -> TokenStream {
  let len = parse_macro_input!(input as Expr);

  TokenStream::from(quote! { advanced += #len; })
}

/// Moves the parser to a new state.
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Ident);
  let state = format_ident!("STATE_{}", definition.to_string().to_uppercase());

  TokenStream::from(quote! {
     if self.state != STATE_ERROR {
       self.state = #state;
     }
  })
}

/// Go to the next iteration of the parser
pub fn next() -> TokenStream { TokenStream::from(quote! { break 'state; }) }

/// Marks the parser as suspended, waiting for more data.
pub fn suspend() -> TokenStream {
  TokenStream::from(quote! {
    parsing = false;
    break 'state;
  })
}

/// Marks the parsing a failed, setting a error code and and error message.
pub fn fail(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as FailureRequest);
  let error = format_ident!("ERROR_{}", definition.error);
  let message = definition.message;

  TokenStream::from(quote! {
    self.fail(#error, #message);
    break 'parser;
  })
}
