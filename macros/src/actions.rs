use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_str, Arm, Expr, Ident};

use crate::{
  definitions::{init_constants, METHODS},
  native::callback_native,
  parsing::{Failure, IdentifiersWithExpr, StringLength},
  wasm::callback_wasm,
};

/// Takes a state name and returns its state variant, which is its uppercased
/// version.
fn format_state(ident: &Ident) -> Ident { format_ident!("STATE_{}", ident.to_string().to_uppercase()) }

/// Returns the length of an input string.
pub fn string_length(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as StringLength);

  let len = definition.string.value().len() + definition.modifier;

  TokenStream::from(quote! { #len })
}

// Marks a certain number of characters as used.
pub fn advance(input: TokenStream) -> TokenStream {
  let name = parse_macro_input!(input as Ident);

  TokenStream::from(quote! { #name })
}

/// Moves the parsers to a new state and marks a certain number of characters as
/// used.
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let state = format_state(&definition.identifier);

  #[cfg(debug_assertions)]
  {
    if let Some(expr) = definition.expr {
      TokenStream::from(quote! { parser.move_to(#state, #expr) })
    } else {
      TokenStream::from(quote! { parser.move_to(#state, 1) })
    }
  }

  #[cfg(not(debug_assertions))]
  {
    if let Some(expr) = definition.expr {
      TokenStream::from(quote! {
        {
          parser.state = #state;
          #expr
        }
      })
    } else {
      TokenStream::from(quote! {
        {
          parser.state = #state;
          1
        }
      })
    }
  }
}

/// Marks the parsing a failed, setting a error code and and error message.
pub fn fail(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Failure);
  let error = format_ident!("ERROR_{}", definition.error);
  let message = definition.message;

  TokenStream::from(quote! { parser.fail(#error, #message) })
}

/// Tries to detect the longest prefix of the data matching the provided
/// selector.
///
/// The `consumed` variable will contain the length of the prefix.
///
/// If all input data matched the selector, the parser will pause to allow eager
/// parsing, in this case the parser must be resumed.
pub fn consume(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Ident);
  let ident = definition.to_string();
  let name = ident.as_str();

  let valid_tables = [
    "digit",
    "hex_digit",
    "token",
    "token_value",
    "token_value_quoted",
    "url",
    "ws",
  ];

  let table: Ident = if valid_tables.contains(&name) {
    format_ident!("{}_TABLE", name.to_uppercase())
  } else {
    panic!("Unsupported consumed type {}", name)
  };

  TokenStream::from(quote! {
    let max = data.len();
    let mut consumed = max;

    // Future: SIMD checks 8 by 8?
    for i in 0..max {
      if !#table[data[i] as usize] {
        consumed = i;
        break;
      }
    }

    if(consumed == max) {
      return SUSPEND;
    }
  })
}

/// Invokes one of the user defined callbacks, eventually attaching some view of
/// the data (via pointer and length). If the callback errors, the operation is
/// NOT interrupted. This call will also append the location information to
/// the offsets.
pub fn callback(input: TokenStream, target: &str, return_on_fail: bool) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let target = format_ident!("{}", target);

  let native = callback_native(&definition, &target);
  let wasm = callback_wasm(&definition, &target);

  let error_handling = if return_on_fail {
    quote!({ if #target.state == STATE_ERROR { return 0 }; })
  } else {
    quote!()
  };

  TokenStream::from(quote! {
    #[cfg(not(target_family = "wasm"))]
    {
      #native
      #error_handling
    }

    #[cfg(target_family = "wasm")]
    {
      #wasm
      #error_handling
    }
  })
}

/// Marks the parser as suspended, waiting for more data.
pub fn suspend() -> TokenStream { TokenStream::from(quote! { SUSPEND }) }

/// Maps a string method to its integer value (which is the enum definition
/// index).
pub fn find_method(input: TokenStream) -> TokenStream {
  let identifier = parse_macro_input!(input as Expr);
  init_constants();

  let methods: Vec<_> = METHODS
    .get()
    .unwrap()
    .iter()
    .enumerate()
    .map(|(i, x)| {
      let matcher = x
        .chars()
        .map(|b| format!("b'{}'", b))
        .collect::<Vec<String>>()
        .join(", ");

      if x == "CONNECT" {
        parse_str::<Arm>(&format!("[{}] => {{ parser.is_connect = true; {} }}", matcher, i)).unwrap()
      } else {
        parse_str::<Arm>(&format!("[{}] => {{ {} }}", matcher, i)).unwrap()
      }
    })
    .collect();

  TokenStream::from(quote! {
    let method = match #identifier {
      #(#methods),*
      _ => {
        return fail!(UNEXPECTED_CHARACTER, "Invalid method")
      }
    };
  })
}
