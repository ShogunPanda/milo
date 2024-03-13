use quote::{format_ident, quote};
use regex::{Captures, Regex};

use crate::{definitions::CALLBACKS, parsing::IdentifiersWithExpr};

pub fn callback_wasm(
  definition: &IdentifiersWithExpr,
  return_on_error: bool,
  use_self: bool,
) -> proc_macro2::TokenStream {
  let callback = &definition.identifier;
  let callback_name = callback.to_string();

  let parser = if use_self {
    format_ident!("self")
  } else {
    format_ident!("parser")
  };

  // Prepopulate the message without runtime format
  let callback_no_return_number = format!("Callback for {} must return a number.", callback_name);
  let callback_throw = format!("Callback for {} has thrown an error.", callback_name);

  let validate_wasm = if return_on_error {
    quote! {
      match ret {
        Ok(value) => {
          match value.as_f64() {
            Some(number) => number as isize,
            None => {
              return #parser.fail(ERROR_CALLBACK_ERROR, #callback_no_return_number);
            }
          }
        }
        Err(err) => {
          #parser.callback_error.replace(err);
          return #parser.fail(ERROR_CALLBACK_ERROR, #callback_throw);
        }
      }
    }
  } else {
    quote! {
      match ret {
        Ok(value) => {
          match value.as_f64() {
            Some(number) => number as isize,
            None => {
              let _ = #parser.fail(ERROR_CALLBACK_ERROR, #callback_no_return_number);
              0 as isize
            }
          }
        }
        Err(err) => {
          0 as isize
        }
      }
    }
  };

  let invocation = if let Some(length) = &definition.expr {
    quote! {
      {
        let ret = #parser.callbacks.#callback.call2(&JsValue::NULL, &JsValue::from(#parser.position.get() as usize), &JsValue::from(#length));
        #validate_wasm
      }
    }
  } else {
    quote! {
      {
        let ret = #parser.callbacks.#callback.call0(&JsValue::NULL);
        #validate_wasm
      }
    }
  };

  let error_message = format!("Callback {} failed with non zero return value.", callback_name);
  let error_handling = if return_on_error {
    quote! {
      return #parser.fail(ERROR_CALLBACK_ERROR, #error_message);
    }
  } else {
    quote! {
      let _ = #parser.fail(ERROR_CALLBACK_ERROR, #error_message);
    }
  };

  quote! {
    let invocation_result = #invocation;

    if invocation_result != 0 {
      #error_handling
    }
  }
}

/// Generates all parser callbacks.
pub fn generate_callbacks_wasm() -> proc_macro2::TokenStream {
  let callbacks: Vec<_> = unsafe {
    CALLBACKS
      .get()
      .unwrap()
      .iter()
      .map(|x| format_ident!("{}", x))
      .collect()
  };

  let snake_matcher = Regex::new(r"_([a-z])").unwrap();

  let setters: Vec<_> = unsafe {
    CALLBACKS
      .get()
      .unwrap()
      .iter()
      .map(|name| {
        let lowercase = format!("set_{}", name.to_lowercase());
        let fn_name = format_ident!("{}", lowercase);
        let cb_name = format_ident!("{}", name);
        let js_name = snake_matcher.replace_all(lowercase.as_str(), |captures: &Captures| captures[1].to_uppercase());
        let error_message = format!("The callback for {} must be a function or a falsy value.", js_name);
        quote! {
          #[wasm_bindgen(js_name=#js_name)]
          pub fn #fn_name(&mut self, cb: Function) -> Result<(), JsValue> {
            if cb.is_falsy() {
              Function::new_no_args("return 0");
              return Ok(())
            } else if !cb.is_function() {
              return Err(
                js_sys::Error::new(#error_message).into()
              );
            }

            self.callbacks.#cb_name = cb;
            Ok(())
          }
        }
      })
      .collect()
  };

  quote! {
    #[cfg(target_family = "wasm")]
    #[repr(C)]
    #[derive(Clone, Debug)]
    pub struct Callbacks {
      #( pub #callbacks: Function),*
    }

    #[cfg(target_family = "wasm")]
    impl Callbacks {
      fn new() -> Callbacks {
        let noop = Function::new_no_args("return 0");

        Callbacks {
          #( #callbacks: noop.clone() ),*
        }
      }
    }

    #[cfg(target_family = "wasm")]
    #[wasm_bindgen]
    impl Parser {
      #(#setters)*
    }
  }
}
