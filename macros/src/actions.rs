use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_str, Arm, Expr, Ident};

use crate::{
  definitions::{init_constants, METHODS, STATES},
  native::callback_native,
  parsing::{Failure, IdentifierWithExpr, StringLength},
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
  let len = parse_macro_input!(input as Expr);

  TokenStream::from(quote! { advanced += #len; })
}

/// Moves the parsers to a new state
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
      break 'parser;
    }
  })
}

/// Invokes one of the user defined callbacks, eventually attaching some view of
/// the data (via pointer and length).
pub fn callback(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifierWithExpr);
  let native = callback_native(&definition);
  let wasm = callback_wasm(&definition);

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
    not_suspended = false;
    break 'state;
  })
}

/// Go to the next iteration of the parser
pub fn r#return() -> TokenStream { TokenStream::from(quote! { break 'state; }) }

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
        parse_str::<Arm>(&format!(
          "[{}] => {{ self.is_connect = true; self.method = {}; }}",
          matcher, i
        ))
        .unwrap()
      } else {
        parse_str::<Arm>(&format!("[{}] => {{ self.method = {}; }}", matcher, i)).unwrap()
      }
    })
    .collect();

  TokenStream::from(quote! {
    match #identifier {
      #(#methods),*
      _ => {
        fail!(UNEXPECTED_CHARACTER, "Invalid method");
      }
    };
  })
}

pub fn process_state() -> TokenStream {
  let states = &unsafe { STATES.get().unwrap() };

  // Uncomment this when trying to debug why compilation failes
  // eprintln!(
  //   "{}",
  //   states.iter().map(|s| format!("STATE_{}\n", s.0)).collect::<String>()
  // );

  let states: Vec<_> = states
    .iter()
    .map(|s| parse_str::<Arm>(&format!("STATE_{} => {{ {} }}", s.0, s.1)).unwrap())
    .collect();

  TokenStream::from(quote! {
    match self.state {
      #(#states),*
      _ => {
        fail!(UNEXPECTED_STATE, "Invalid state");
      }
    }
  })
}
