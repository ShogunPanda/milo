use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::{Captures, Regex};
use syn::{parse_macro_input, Ident};

use crate::parsing::{IdentifiersWithExpr, Property};

// Handles a callback.
pub fn callback_wasm(definition: &IdentifiersWithExpr, target: &Ident) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;
  let callback_name = callback.to_string();
  let callback_value = format_ident!("CALLBACK_{}", callback_name.to_uppercase());

  let invocation = if let Some(length) = &definition.expr {
    quote! { unsafe { run_callback(crate::#callback_value, #target.ptr, #target.position, #length) }; }
  } else {
    quote! { unsafe { run_callback(crate::#callback_value, #target.ptr, 0, 0) }; }
  };

  quote! {
    #invocation;
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
