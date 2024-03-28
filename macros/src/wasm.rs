use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, Ident};

use crate::parsing::{IdentifiersWithExpr, Property};

// Handles a callback.
pub fn callback_wasm(definition: &IdentifiersWithExpr, target: &Ident) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;
  let callback_name = callback.to_string();
  let callback_value = format_ident!("CALLBACK_{}", callback_name.to_uppercase());

  let invocation = if let Some(length) = &definition.expr {
    quote! { run_callback(crate::#callback_value, #target.ptr, #target.position, #length); }
  } else {
    quote! { run_callback(crate::#callback_value, #target.ptr, 0, 0); }
  };

  quote! {
    #invocation;
  }
}

// Generates a getter.
pub fn wasm_getter(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Property);
  let property = definition.property;
  let fn_name = format_ident!("get_{}", &property.to_string());
  let getter = definition.getter;
  let return_type = definition.r#type;

  TokenStream::from(quote! {
    /// Gets the parser #property.
    #[wasm_bindgen(js_name = #getter)]
    pub fn #fn_name(parser: *const c_void) -> #return_type { unsafe { (*(parser as *const Parser)).#property } }
  })
}
