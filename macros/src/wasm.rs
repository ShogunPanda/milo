use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::{Captures, Regex};
use syn::parse_macro_input;

use crate::parsing::{IdentifiersWithExpr, Property};

// Handles a callback.
pub fn callback_wasm(definition: &IdentifiersWithExpr, return_on_error: bool) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;
  let callback_name = callback.to_string();
  let callback_value = format_ident!("CALLBACK_{}", callback_name.to_uppercase());

  let lowercase = callback_name.to_lowercase();
  let callback_name_camelcase = Regex::new(r"_([a-z])")
    .unwrap()
    .replace_all(lowercase.as_str(), |captures: &Captures| captures[1].to_uppercase());

  // Prepopulate the message without runtime format
  let callback_throw = format!("Callback for {} has thrown an error.", callback_name_camelcase);

  let validate_wasm = if return_on_error {
    quote! {
      match ret {
        Ok(value) => {
          value
        }
        Err(err) => {
          parser.callback_error.replace(err);
          return fail(parser, ERROR_CALLBACK_ERROR, #callback_throw);
        }
      }
    }
  } else {
    quote! {
      match ret {
        Ok(value) => {
          value
        }
        Err(err) => {
          parser.callback_error.replace(err);
          0 as isize
        }
      }
    }
  };

  let invocation = if let Some(length) = &definition.expr {
    quote! {
      {
        let ret = run_callback(parser.wasm_ptr.get(), crate::#callback_value, parser.position.get(), #length);
        #validate_wasm
      }
    }
  } else {
    quote! {
      {
        let ret = run_callback(parser.wasm_ptr.get(), crate::#callback_value, 0, 0);
        #validate_wasm
      }
    }
  };

  let error_message = format!("Callback {} failed with non zero return value.", callback_name);
  let error_handling = if return_on_error {
    quote! {
      return fail(parser, ERROR_CALLBACK_ERROR, #error_message);
    }
  } else {
    quote! {
      let _ = fail(parser, ERROR_CALLBACK_ERROR, #error_message);
    }
  };

  quote! {
    let invocation_result = #invocation;

    if invocation_result != 0 {
      #error_handling
    }
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
    #[wasm_bindgen(js_name = #getter)]
    pub fn #fn_name(raw: *mut c_void) -> #return_type {
      let parser = unsafe { Box::from_raw(raw as *mut Parser) };
      let value = parser.#property.get();
      Box::into_raw(parser);

      value
    }
  })
}
