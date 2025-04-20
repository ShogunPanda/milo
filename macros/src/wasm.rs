use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::{Captures, Regex};
use syn::parse_macro_input;

use crate::{
  definitions::CALLBACKS,
  parsing::{IdentifierWithExpr, Property},
};

// Handles a callback.
pub fn callback_wasm(definition: &IdentifierWithExpr) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;

  if let Some(length) = &definition.expr {
    quote! { unsafe { #callback(self.ptr, self.position, #length) }; }
  } else {
    quote! { unsafe { #callback(self.ptr, 0, 0) }; }
  }
}

// Generates a getter.
pub fn wasm_getter(input: TokenStream) -> TokenStream {
  let snake_matcher = Regex::new(r"([A-Z])").unwrap();

  let definition = parse_macro_input!(input as Property);
  let property = definition.property;
  let fn_name = format_ident!(
    "{}",
    snake_matcher.replace_all(&definition.getter.to_string().as_str(), |captures: &Captures| {
      format!("_{}", captures[1].to_lowercase())
    })
  );

  let return_type = definition.r#type;

  TokenStream::from(quote! {
    /// Gets the parser #property.
    #[no_mangle]
    pub fn #fn_name(parser: *const c_void) -> #return_type { unsafe { (*(parser as *const Parser)).#property } }
  })
}

/// Generates all parser callbacks.
pub fn link_callbacks() -> TokenStream {
  let callbacks: Vec<_> = CALLBACKS
    .get()
    .unwrap()
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();

  TokenStream::from(quote! {
    #(fn #callbacks(parser: *mut c_void, _at: usize, _len: usize);)*
  })
}
