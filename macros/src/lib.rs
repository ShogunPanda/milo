#[macro_use]
extern crate lazy_static;

mod parsing;

use std::ffi::c_uchar;
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;

use indexmap::IndexSet;
use parsing::{Failure, Identifiers, IdentifiersWithExpr, State, StringLength};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_str, Arm, Expr, Ident, ItemConst, LitByte, LitChar, LitInt, LitStr};

// Global state variables for later use
lazy_static! {
  static ref METHODS: Mutex<Vec<String>> = {
    let mut absolute_path = Path::new(file!()).parent().unwrap().to_path_buf();
    absolute_path.push("methods.yml");
    let f = File::open(absolute_path.to_str().unwrap()).unwrap();
    let methods = serde_yaml::from_reader(f).unwrap();

    Mutex::new(methods)
  };
  static ref ERRORS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref CALLBACKS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref STATES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
}

/// Takes a state name and returns its state variant, which is its uppercased
/// version.
fn format_state(ident: &Ident) -> Ident { format_ident!("{}", ident.to_string().to_uppercase()) }

// #region definitions
/// Adds one or more new error codes.
#[proc_macro]
pub fn errors(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut errors = ERRORS.lock().unwrap();

  for error in definition.identifiers {
    errors.insert(error.to_string().to_uppercase());
  }

  TokenStream::new()
}

/// Adds one or more new callback.
#[proc_macro]
pub fn callbacks(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut callbacks = CALLBACKS.lock().unwrap();

  for cb in definition.identifiers {
    callbacks.insert(cb.to_string());
  }

  TokenStream::new()
}

/// Adds time measurement to a code block.
#[proc_macro]
pub fn measure(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as State);
  let name = definition.name.to_string();
  let statements = definition.statements;

  TokenStream::from(quote! {
    {
      let mut start = SystemTime::now();

      let res = { #(#statements)* };

      let duration = SystemTime::now().duration_since(start).unwrap().as_nanos();

      println!("[milo::debug] {} completed in {} ns", #name, duration);
      res
    }
  })
}

/// Defines a new state.
#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as State);
  let name = definition.name;
  let function = format_ident!("state_{}", name);
  let statements = definition.statements;

  STATES.lock().unwrap().insert(name.to_string().to_uppercase());

  TokenStream::from(quote! {
    #[inline(always)]
    fn #function (parser: &mut Parser, data: &[c_uchar]) -> isize {
      let mut data = data;
      #(#statements)*
    }
  })
}
// #endregion definitions

// #region matchers
/// Matches a character.
#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream {
  let character = parse_macro_input!(input as LitChar);
  let byte = LitByte::new(c_uchar::try_from(character.value()).unwrap(), character.span());

  TokenStream::from(quote! { #byte })
}

/// Matches a digit in base 10.
#[proc_macro]
pub fn digit(_input: TokenStream) -> TokenStream { TokenStream::from(quote! { 0x30..=0x39 }) }

/// Matches a digit in base 16.
#[proc_macro]
pub fn hex_digit(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { 0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66 })
}

/// Matches a string in case sensitive way.
#[proc_macro]
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
#[proc_macro]
pub fn case_insensitive_string(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as LitStr);

  let bytes: Vec<_> = definition
    .value()
    .bytes()
    .map(|b| parse_str::<Expr>(&format!("{} | {}", b.to_ascii_uppercase(), b.to_ascii_lowercase())).unwrap())
    .collect();

  TokenStream::from(quote! { [#(#bytes),*, ..] })
}

/// Matches a "CR LF" sequence.
#[proc_macro]
pub fn crlf(_: TokenStream) -> TokenStream { TokenStream::from(quote! { [b'\r', b'\n', ..] }) }

/// Matches a "CR LF CR LF" sequence.
#[proc_macro]
pub fn double_crlf(_: TokenStream) -> TokenStream { TokenStream::from(quote! { [b'\r', b'\n', b'\r', b'\n', ..] }) }

/// Matches a token according to RFC 9110 section 5.6.2 and RFC 5234 appendix
/// B.1.
///
/// DIGIT | ALPHA | OTHER_TOKENS
/// DIGIT = 0x30 - 0x39
/// ALPHA = 0x41-0x5A, 0x61 - 0x7A
/// OTHER_TOKENS = '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' |
/// '^' | '_' | '`' | '|' | '~'
#[proc_macro]
pub fn token(_input: TokenStream) -> TokenStream {
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
#[proc_macro]
pub fn token_value(_input: TokenStream) -> TokenStream {
  // RFC 9112 section 4
  //
  TokenStream::from(quote! { b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff })
}

/// Matches a method according to HTTP and RTSP standards.
///
/// HTTP = https://www.iana.org/assignments/http-methods (RFC 9110 section 16.1.1)
/// RTSP = RFC 7826 section 7.1
#[proc_macro]
pub fn method(input: TokenStream) -> TokenStream {
  let methods = METHODS.lock().unwrap();

  let output: Vec<_> = if input.is_empty() {
    methods.iter().map(|x| quote! { string!(#x) }).collect()
  } else {
    let identifier = parse_macro_input!(input as Ident);
    methods.iter().map(|x| quote! { #identifier @ string!(#x) }).collect()
  };

  TokenStream::from(quote! { #(#output)|* })
}

/// Matches a method according to RFC 3986 appendix A and RFC 5234 appendix B.1.
///
/// DIGIT | ALPHA | OTHER_UNRESERVED_AND_RESERVED
/// DIGIT = 0x30 - 0x39
/// ALPHA = 0x41-0x5A, 0x61 - 0x7A
/// OTHER_UNRESERVED_AND_RESERVED = '-' | '.' | '_' | '~' | ':' | '/' | '?' |
/// '#' | '[' | ']' | '@' | '!' | '$' | '&' | ''' | '(' | ')' | '*' | '+' | ','
/// | ';' | '=' | '%'
#[proc_macro]
pub fn url(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'-' | b'.' | b'_' | b'~' | b':' | b'/' | b'?' | b'#' | b'[' | b']' | b'@' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b'%'
  })
}

/// Matches any sequence of N characters. This is used as failure state when at
/// least N characters are available.
#[proc_macro]
pub fn otherwise(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as LitInt);
  let tokens = definition.base10_parse::<isize>().unwrap();
  let quotes: Vec<_> = (0..tokens).map(|x| format_ident!("_u{}", format!("{}", x))).collect();

  TokenStream::from(quote! { [ #(#quotes),*, .. ] })
}
// #endregion matchers

// #region actions
/// Returns the length of an input string.
#[proc_macro]
pub fn string_length(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as StringLength);

  let len = definition.string.value().len() as isize + definition.modifier;

  TokenStream::from(quote! { #len })
}

#[proc_macro]
/// Moves the parsers to a new state and marks a certain number of characters as
/// used.
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let state = format_state(&definition.identifier);

  #[cfg(debug_assertions)]
  {
    if let Some(expr) = definition.expr {
      TokenStream::from(quote! { parser.move_to(State::#state, (#expr) as isize) })
    } else {
      TokenStream::from(quote! { parser.move_to(State::#state, 1) })
    }
  }

  #[cfg(not(debug_assertions))]
  {
    if let Some(expr) = definition.expr {
      TokenStream::from(quote! {
        {
          parser.state = State::#state;
          (#expr) as isize
        }
      })
    } else {
      TokenStream::from(quote! {
        {
          parser.state = State::#state;
          1
        }
      })
    }
  }
}

/// Marks the parsing a failed, setting a error code and and error message.
#[proc_macro]
pub fn fail(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Failure);
  let error = definition.error;
  let message = definition.message;

  if let Expr::Lit(_) = message {
    TokenStream::from(quote! { parser.fail_str(Error::#error, #message) })
  } else {
    TokenStream::from(quote! { parser.fail(Error::#error, #message) })
  }
}

/// Tries to detect the longest prefix of the data matching the provided
/// selector.
///
/// The `consumed` variable will contain the length of the prefix.
///
/// If all input data matched the selector, the parser will pause to allow eager
/// parsing, in this case the parser must be resumed.

#[proc_macro]
pub fn consume(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Expr);

  TokenStream::from(quote! {
    let mut consumed = 0;
    let max = data.len();

    while consumed < max {
      if let #definition = data[consumed] {
        consumed += 1;
      } else {
        break
      }
    }

    if(consumed == max) {
      return SUSPEND;
    }
  })
}

/// Invokes one of the user defined callbacks, eventually attaching some view of
/// the data (via pointer and length).
#[proc_macro]
pub fn callback(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let callback = definition.identifier;
  let callback_name = callback.to_string();

  let invocation = if let Some(length) = definition.expr {
    quote! {
      (parser.callbacks.#callback)(parser, data.as_ptr(), (#length) as usize)
    }
  } else {
    quote! {
      (parser.callbacks.#callback)(parser, ptr::null(), 0)
    }
  };

  TokenStream::from(quote! {
    let invocation_result = #invocation;

    if invocation_result != 0 {
      return parser.fail(
        Error::CALLBACK_ERROR,
        format!("Callback {} failed with return value {}.", #callback_name, invocation_result),
      );
    }
  })
}

/// Marks the parser as suspended, waiting for more data.
#[proc_macro]
pub fn suspend(_input: TokenStream) -> TokenStream { TokenStream::from(quote! { SUSPEND }) }

/// Maps a string method to its integer value (which is the enum definition
/// index).
#[proc_macro]
pub fn find_method(input: TokenStream) -> TokenStream {
  let identifier = parse_macro_input!(input as Expr);

  let methods: Vec<_> = METHODS
    .lock()
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
          "[{}, ..] => {{ parser.is_connect_request = 1; {} }}",
          matcher, i
        ))
        .unwrap()
      } else {
        parse_str::<Arm>(&format!("[{}, ..] => {{ {} }}", matcher, i)).unwrap()
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
// #endregion actions

// #region generators
/// Returns the initial state of the parser, which is the first defined state.
#[proc_macro]
pub fn initial_state(_input: TokenStream) -> TokenStream {
  let initial_state = format_ident!("{}", STATES.lock().unwrap()[0]);

  TokenStream::from(quote! { State::#initial_state })
}

/// Returns a match statement which executes the block of the parser's current
/// state.
#[proc_macro]
pub fn apply_state(_input: TokenStream) -> TokenStream {
  // Generate all the branches
  let states_arms: Vec<_> = STATES
    .lock()
    .unwrap()
    .iter()
    .map(|x| {
      parse_str::<Arm>(&format!(
        "State::{} => state_{}(self, current)",
        x,
        x.to_string().to_lowercase()
      ))
      .unwrap()
    })
    .collect();

  TokenStream::from(quote! {
    match self.state {
      State::FINISH => 0,
      State::ERROR => 0,
      #(#states_arms),*,
    }
  })
}

/// Translates a state enum variant to a string.
#[proc_macro]
pub fn c_match_state_string(_input: TokenStream) -> TokenStream {
  let states_to_string_arms: Vec<_> = STATES
    .lock()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    match self.state {
      State::FINISH => "FINISH",
      State::ERROR => "ERROR",
      #(#states_to_string_arms),*
    }.into()
  })
}

/// Translates a error code enum variant to a string.
#[proc_macro]
pub fn c_match_error_code_string(_input: TokenStream) -> TokenStream {
  let error_to_string_arms: Vec<_> = ERRORS
    .lock()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Error::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    match &self.error_code {
      Error::NONE => "NONE",
      Error::UNEXPECTED_DATA => "UNEXPECTED_DATA",
      Error::UNEXPECTED_EOF => "UNEXPECTED_EOF",
      Error::CALLBACK_ERROR => "CALLBACK_ERROR",
      #(#error_to_string_arms),*
    }.into()
  })
}

// #endregion generators

/// Generates all parser enum and structs out of methods state, errors and
/// callbacks definitions.
#[proc_macro]
pub fn generate_parser_types(_input: TokenStream) -> TokenStream {
  let methods_ref = METHODS.lock().unwrap();
  let states_ref = STATES.lock().unwrap();
  let errors_ref = ERRORS.lock().unwrap();

  let methods: Vec<_> = methods_ref
    .iter()
    .map(|x| format_ident!("{}", x.replace('-', "_")))
    .collect();

  let states: Vec<_> = states_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let errors: Vec<_> = errors_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let methods_consts: Vec<_> = methods_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const METHOD_{}: isize = {};", x.replace('-', "_"), i)).unwrap())
    .collect();

  let states_consts: Vec<_> = states_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const STATES_{}: isize = {};", x, i)).unwrap())
    .collect();

  let errors_consts: Vec<_> = errors_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const ERROR_{}: isize = {};", x, i)).unwrap())
    .collect();

  let callbacks: Vec<_> = CALLBACKS
    .lock()
    .unwrap()
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();

  let output = quote! {
    fn noop_internal(_parser: &mut Parser, _data: *const c_uchar, _len: usize) -> isize {
      0
    }

    pub const SUSPEND: isize = isize::MIN;

    #(#errors_consts)*

    #(#methods_consts)*

    #(#states_consts)*

    #[no_mangle]
    pub type Callback = fn (&mut Parser, *const c_uchar, usize) -> isize;

    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum Method {
      #(#methods),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum State {
      #(#states),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum Error {
      #(#errors),*
    }

    #[repr(C)]
    #[derive(Copy, Clone, Debug)]
    pub struct Callbacks {
      #( pub #callbacks: Callback),*
    }

    impl Callbacks {
      fn new() -> Callbacks {
        Callbacks {
          #( #callbacks: noop_internal ),*
        }
      }
    }
  };

  TokenStream::from(output)
}
