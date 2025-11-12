use std::ffi::c_uchar;

use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, Ident, LitByte, LitChar, LitInt, LitStr, parse_macro_input, parse_str};

use crate::structs::StringLength;

/// Matches a "CR LF" sequence.
pub fn crlf_new() -> TokenStream {
  TokenStream::from(quote! {
    data.len() >= 2 && data.starts_with(b"\r\n")
  })
}

/// Matches a token according to RFC 9110 section 5.6.2 and RFC 5234 appendix
/// B.1.
///
/// DIGIT | ALPHA | OTHER_TOKENS
/// DIGIT = 0x30 - 0x39
/// ALPHA = 0x41-0x5A, 0x61 - 0x7A
/// OTHER_TOKENS = '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' |
/// '^' | '_' | '`' | '|' | '~'
pub fn token_new() -> TokenStream { TokenStream::from(quote! {!data.is_empty() && TOKEN_TABLE[data[0] as usize]}) }

/// Matches a character.
pub fn char(input: TokenStream) -> TokenStream {
  let character = parse_macro_input!(input as LitChar);
  let byte = LitByte::new(c_uchar::try_from(character.value()).unwrap(), character.span());

  TokenStream::from(quote! { #byte })
}

/// Returns the length of an input string.
pub fn string_length(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as StringLength);

  let len = definition.string.value().len() + definition.modifier;

  TokenStream::from(quote! { #len })
}

/// Matches a string in case sensitive way.
pub fn string(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as LitStr);
  let bytes: Vec<_> = definition
    .value()
    .bytes()
    .map(|b| LitByte::new(b, definition.span()))
    .collect();

  TokenStream::from(quote! { [#(#bytes),*, ..] })
}

/// Matches a string in case insensitive way.
pub fn case_insensitive_string(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as LitStr);

  let bytes: Vec<_> = definition
    .value()
    .bytes()
    .map(|b| parse_str::<Expr>(&format!("{} | {}", b.to_ascii_uppercase(), b.to_ascii_lowercase())).unwrap())
    .collect();

  TokenStream::from(quote! { [#(#bytes),*, ..] })
}

// #[proc_macro]
// pub fn case_insensitive_string(input: TokenStream) -> TokenStream {
// let definition = parse_macro_input!(input as LitStr);
// let value = definition.value();
// let bytes = value.as_bytes();
// let len = bytes.len();
//
// Genera confronti unrolled
// let comparisons: Vec<_> = bytes.iter().enumerate().map(|(i, &byte)| {
// let lowercase = byte.to_ascii_lowercase();
//
// if byte.is_ascii_alphabetic() {
// Per lettere: lowercase e confronta
// quote! { (data[#i] | 0x20) == #lowercase }
// } else {
// Per non-lettere: confronto diretto
// quote! { data[#i] == #lowercase }
// }
// }).collect();
//
// TokenStream::from(quote! {
// {
// data.len() >= #len && #(#comparisons)&&*
// }
// })
// }
//

/// Matches a "CR LF" sequence.
pub fn crlf() -> TokenStream { TokenStream::from(quote! { [b'\r', b'\n', ..] }) }

/// Matches a "CR LF CR LF" sequence.
pub fn double_crlf() -> TokenStream { TokenStream::from(quote! { [b'\r', b'\n', b'\r', b'\n', ..] }) }

/// Matches a token according to RFC 9110 section 5.6.2 and RFC 5234 appendix
/// B.1.
///
/// DIGIT | ALPHA | OTHER_TOKENS
/// DIGIT = 0x30 - 0x39
/// ALPHA = 0x41-0x5A, 0x61 - 0x7A
/// OTHER_TOKENS = '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' |
/// '^' | '_' | '`' | '|' | '~'
pub fn token() -> TokenStream {
  TokenStream::from(quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
  })
}

/// Matches a token according to RFC 9112 section 4.
///
/// HTAB / SP / VCHAR / obs-text
pub fn token_value() -> TokenStream {
  // RFC 9112 section 4
  //
  TokenStream::from(quote! { b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff })
}

/// Matches a method according to RFC 3986 appendix A and RFC 5234 appendix B.1.
///
/// DIGIT | ALPHA | OTHER_UNRESERVED_AND_RESERVED
/// DIGIT = 0x30 - 0x39
/// ALPHA = 0x41-0x5A, 0x61 - 0x7A
/// OTHER_UNRESERVED_AND_RESERVED = '-' | '.' | '_' | '~' | ':' | '/' | '?' |
/// '#' | '[' | ']' | '@' | '!' | '$' | '&' | ''' | '(' | ')' | '*' | '+' | ','
/// | ';' | '=' | '%'
pub fn url() -> TokenStream {
  TokenStream::from(quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'-' | b'.' | b'_' | b'~' | b':' | b'/' | b'?' | b'#' | b'[' | b']' | b'@' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b'%'
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

/// Matches any sequence of N characters. This is used as failure state when at
/// least N characters are available.
pub fn otherwise(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as LitInt);
  let tokens = definition.base10_parse::<usize>().unwrap();
  let quotes: Vec<_> = (0..tokens).map(|x| format_ident!("_u{}", format!("{}", x))).collect();

  TokenStream::from(quote! { [ #(#quotes),*, .. ] })
}
