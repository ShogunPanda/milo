use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, parse_macro_input};

use crate::{
  native,
  structs::{Failure, IdentifierWithExpr},
  wasm,
};

/// Takes a state name and returns its state variant, which is its uppercased
/// version.
fn format_state(ident: &Ident) -> Ident { format_ident!("STATE_{}", ident.to_string().to_uppercase()) }

// Marks a certain number of characters as used.
pub fn advance(input: TokenStream) -> TokenStream {
  let len = parse_macro_input!(input as Expr);

  TokenStream::from(quote! { advanced += #len; })
}

// Define the body for a state.
pub fn state(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Ident);
  let state = format_state(&definition);

  TokenStream::from(quote! { #state })
}

/// Moves the parser to a new state.
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Ident);
  let state = format_state(&definition);

  TokenStream::from(quote! { self.state = #state; })
}

/// Marks the parsing a failed, setting a error code and and error message.
pub fn fail(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Failure);
  let error = format_ident!("ERROR_{}", definition.error);
  let message = definition.message;

  TokenStream::from(quote! {
    self.fail(#error, #message);
    break 'parser;
  })
}

/// Invokes one of the user defined callbacks, eventually attaching some view of
/// the data (via pointer and length).
pub fn callback(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifierWithExpr);
  let native = native::callback(&definition);
  let wasm = wasm::callback(&definition);

  TokenStream::from(quote! {
    #[cfg(not(target_family = "wasm"))]
    #native

    #[cfg(target_family = "wasm")]
    #wasm
  })
}

/// Marks the parser as suspended, waiting for more data.
pub fn suspend() -> TokenStream {
  TokenStream::from(quote! {
    parsing = false;
    break 'state;
  })
}

/// Go to the next iteration of the parser
pub fn parse_next() -> TokenStream { TokenStream::from(quote! { break 'state; }) }
