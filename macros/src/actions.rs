use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_str, Arm, Expr, Ident};

use crate::{
  definitions::{init_constants, METHODS, OFFSETS, VALUES_OFFSETS},
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

  TokenStream::from(quote! { #name as isize })
}

/// Moves the parsers to a new state and marks a certain number of characters as
/// used.
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let state = format_state(&definition.identifier);

  #[cfg(debug_assertions)]
  {
    if let Some(expr) = definition.expr {
      TokenStream::from(quote! { move_to(parser, #state, (#expr) as isize) })
    } else {
      TokenStream::from(quote! { move_to(parser, #state, 1) })
    }
  }

  #[cfg(not(debug_assertions))]
  {
    if let Some(expr) = definition.expr {
      TokenStream::from(quote! {
        {
          set!(state, #state);
          (#expr) as isize
        }
      })
    } else {
      TokenStream::from(quote! {
        {
          set!(state, #state);
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

  TokenStream::from(quote! { fail(parser, #error, #message) })
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
pub fn callback(input: TokenStream, return_on_error: bool) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let native = callback_native(&definition, return_on_error);
  let wasm = callback_wasm(&definition, return_on_error);
  let name = definition.identifier.to_string().to_uppercase();

  // Check if the offset named after the callbacks (except the "on_" prefix)
  // exists
  let offset = if unsafe { OFFSETS.get().unwrap() }.contains(&name[3..]) {
    let length = definition.expr.unwrap_or(parse_str::<Expr>(&format!("0")).unwrap());
    let offset_name = format_ident!("OFFSET_{}", name[3..]);

    quote! {
      unsafe {
        let offsets = &parser.offsets;

        // Get the current offset and increase it
        let offsets_count = parser.values.add(VALUE_OFFSETS_COUNT).cast::<usize>();
        let mut current = offsets_count.read();
        offsets_count.write(current + 1);

        // Multiply by three since offsets are actually triplets
        current *= 3;

        // Set the offset type, the start and the length
        offsets.add(current).cast::<usize>().write(#offset_name);
        offsets.add(current + 1).cast::<usize>().write(get!(position));
        offsets.add(current + 2).cast::<usize>().write((#length));
      }
    }
  } else {
    quote! {}
  };

  TokenStream::from(quote! {
    #offset

    #[cfg(not(target_family = "wasm"))]
    {
      #native
    }

    #[cfg(target_family = "wasm")]
    {
      #wasm
    }
  })
}

/// Marks the parser as suspended, waiting for more data.
pub fn suspend() -> TokenStream { TokenStream::from(quote! { SUSPEND }) }

/// Maps a string method to its integer value (which is the enum definition
/// index).
pub fn find_method(input: TokenStream) -> TokenStream {
  let identifier = parse_macro_input!(input as Expr);
  let is_connect_offset = unsafe { VALUES_OFFSETS.get().unwrap().get("is_connect").unwrap() };
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
          "[{}] => {{ unsafe {{ parser.values.add({}).cast::<bool>().write(true); }}; {} }}",
          matcher, is_connect_offset, i
        ))
        .unwrap()
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
