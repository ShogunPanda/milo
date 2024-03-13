use quote::{format_ident, quote};

use crate::{definitions::CALLBACKS, parsing::IdentifiersWithExpr};

pub fn callback_native(
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

  let invocation = if let Some(length) = &definition.expr {
    quote! { (#parser.callbacks.#callback.get())(#parser, #parser.position.get() as usize, (#length) as usize) }
  } else {
    quote! { (#parser.callbacks.#callback.get())(#parser, 0, 0) }
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
pub fn generate_callbacks_native() -> proc_macro2::TokenStream {
  let callbacks: Vec<_> = unsafe {
    CALLBACKS
      .get()
      .unwrap()
      .iter()
      .map(|x| format_ident!("{}", x))
      .collect()
  };

  quote! {
    #[cfg(not(target_family = "wasm"))]
    fn noop_internal(_parser: &Parser, _data: usize, _len: usize) -> isize {
      0
    }

    #[cfg(not(target_family = "wasm"))]
    #[repr(C)]
    #[derive(Clone, Debug)]
    pub struct Callbacks {
      #( pub #callbacks: Cell<Callback>),*
    }

    #[cfg(not(target_family = "wasm"))]
    impl Callbacks {
      fn new() -> Callbacks {
        Callbacks {
          #( #callbacks: Cell::new(noop_internal) ),*
        }
      }
    }
  }
}
