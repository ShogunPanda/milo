use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse_macro_input;

use crate::{
  definitions::{VALUES, VALUES_OFFSETS},
  parsing::IdentifiersWithExpr,
};

/// Gets a parser value.
pub fn get(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let offset = unsafe { VALUES_OFFSETS.get().unwrap() }
    .get(&definition.identifier.to_string())
    .unwrap();
  let value_type = format_ident!(
    "{}",
    unsafe { VALUES.get().unwrap() }
      .get(&definition.identifier.to_string())
      .unwrap()
  );

  TokenStream::from(quote! { unsafe { parser.values.add(#offset).cast::<#value_type>().read() } })
}

/// Sets a parser value.
pub fn set(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let offset = unsafe { VALUES_OFFSETS.get().unwrap() }
    .get(&definition.identifier.to_string())
    .unwrap();
  let value = definition.expr;
  let value_type = format_ident!(
    "{}",
    unsafe { VALUES.get().unwrap() }
      .get(&definition.identifier.to_string())
      .unwrap()
  );

  TokenStream::from(quote! { unsafe { parser.values.add(#offset).cast::<#value_type>().write(#value) } })
}

/// Add value to a parser value.
pub fn add(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let offset = unsafe { VALUES_OFFSETS.get().unwrap() }
    .get(&definition.identifier.to_string())
    .unwrap();
  let value = definition.expr;
  let value_type = format_ident!(
    "{}",
    unsafe { VALUES.get().unwrap() }
      .get(&definition.identifier.to_string())
      .unwrap()
  );

  TokenStream::from(quote! { unsafe { *(parser.values.add(#offset).cast::<#value_type>()) += #value } })
}

/// Subtracts value to a parser value.
pub fn sub(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let offset = unsafe { VALUES_OFFSETS.get().unwrap() }
    .get(&definition.identifier.to_string())
    .unwrap();
  let value = definition.expr;
  let value_type = format_ident!(
    "{}",
    unsafe { VALUES.get().unwrap() }
      .get(&definition.identifier.to_string())
      .unwrap()
  );

  TokenStream::from(quote! { unsafe { *(parser.values.add(#offset).cast::<#value_type>()) -= #value } })
}

/// Generate getters for the parser.
pub fn getters() -> TokenStream {
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

      let return_type = format_ident!("{}", raw_return_type);

      quote! {
        // Returns the parser #name.
        pub fn #getter (parser: &Parser) -> #return_type {
          unsafe { parser.values.add(#offset).cast::<#return_type>().read() }
        }
      }
    })
    .collect();

  TokenStream::from(quote! { #(#getters)* })
}
