use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::{Captures, Regex};

use crate::{
  definitions::{VALUES, VALUES_OFFSETS},
  parsing::IdentifiersWithExpr,
};

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
        let ret = run_callback(parser.ptr.get(), crate::#callback_value, get!(position), #length);
        #validate_wasm
      }
    }
  } else {
    quote! {
      {
        let ret = run_callback(parser.ptr.get(), crate::#callback_value, 0, 0);
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

/// Generate getters for the parser.
pub fn wasm_getters() -> TokenStream {
  let values_offsets = unsafe { VALUES_OFFSETS.get().unwrap() };

  let getters: Vec<_> = unsafe { VALUES.get().unwrap() }
    .iter()
    .map(|(name, raw_return_type)| {
      let offset = values_offsets.get(name).unwrap();
      let getter = if name.starts_with("is") || name.starts_with("has") {
        format_ident!("{}", name)
      } else {
        format_ident!("{}_{}", if raw_return_type == "bool" { "is" } else { "get" }, name)
      };

      let snake_matcher = Regex::new(r"_([a-z])").unwrap();

      let js_name = format_ident!(
        "{}",
        snake_matcher.replace_all(&getter.to_string(), |captures: &Captures| captures[1].to_uppercase())
      );

      let return_type = format_ident!("{}", raw_return_type);

      quote! {
        // Returns the parser #name.
        #[wasm_bindgen(js_name = #js_name)]

        // Returns the parser #name.
        pub fn #getter (parser: *mut c_void) -> #return_type {
          unsafe { parser.cast::<Parser>().read().values.add(#offset).cast::<#return_type>().read() }
        }
      }
    })
    .collect();

  TokenStream::from(quote! { #(#getters)* })
}
