#[macro_use]
extern crate lazy_static;

mod parsing;

use std::{ffi::c_uchar, sync::RwLock};
use std::fs::File;
use std::path::Path;

use indexmap::IndexSet;
use parsing::{Failure, Identifiers, IdentifiersWithExpr, State, StringLength};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, parse_str, Arm, Expr, Ident, ItemConst, LitByte, LitChar, LitInt, LitStr};

const SUSPEND: isize = isize::MIN;

lazy_static! {
  static ref METHODS: RwLock<Vec<String>> = {
    let mut absolute_path = Path::new(file!()).parent().unwrap().to_path_buf();
    absolute_path.push("methods.yml");
    let f = File::open(absolute_path.to_str().unwrap()).unwrap();
    let methods = serde_yaml::from_reader(f).unwrap();

    RwLock::new(methods)
  };
  static ref STATES: RwLock<IndexSet<String>> = RwLock::new(IndexSet::new());
  static ref ERRORS: RwLock<IndexSet<String>> = RwLock::new(IndexSet::new());
  static ref VALUES: RwLock<IndexSet<String>> = RwLock::new(IndexSet::new());
  static ref PERSISTENT_VALUES: RwLock<IndexSet<String>> = RwLock::new(IndexSet::new());
  static ref USER_WRITABLE_VALUES: RwLock<IndexSet<String>> = RwLock::new(IndexSet::new());
  static ref CALLBACKS: RwLock<IndexSet<String>> = RwLock::new(IndexSet::new());
}

fn format_state(ident: &Ident) -> Ident { format_ident!("{}", ident.to_string().to_uppercase()) }

// #region definitions
#[proc_macro]
pub fn values(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut values = VALUES.write().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn user_writable_values(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut values = USER_WRITABLE_VALUES.write().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn persistent_values(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut values = PERSISTENT_VALUES.write().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn errors(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut errors = ERRORS.write().unwrap();

  for error in definition.identifiers {
    errors.insert(error.to_string().to_uppercase());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn callbacks(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut callbacks = CALLBACKS.write().unwrap();

  for cb in definition.identifiers {
    callbacks.insert(cb.to_string());
  }

  TokenStream::new()
}

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

#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as State);
  let name = definition.name;
  let function = format_ident!("state_{}", name);
  let statements = definition.statements;

  STATES.write().unwrap().insert(name.to_string().to_uppercase());

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
#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream {
  let character = parse_macro_input!(input as LitChar);
  let byte = LitByte::new(c_uchar::try_from(character.value()).unwrap(), character.span());

  TokenStream::from(quote! { #byte })
}

#[proc_macro]
pub fn digit(_input: TokenStream) -> TokenStream { TokenStream::from(quote! { 0x30..=0x39 }) }

#[proc_macro]
pub fn hex_digit(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { 0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66 })
}

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

#[proc_macro]
pub fn crlf(_: TokenStream) -> TokenStream { TokenStream::from(quote! { [b'\r', b'\n', ..] }) }

#[proc_macro]
pub fn double_crlf(_: TokenStream) -> TokenStream { TokenStream::from(quote! { [b'\r', b'\n', b'\r', b'\n', ..] }) }

#[proc_macro]
pub fn token(_input: TokenStream) -> TokenStream {
  // RFC 9110 section 5.6.2 and RFC 5234 appendix B.1
  // DIGIT = 0x30 - 0x39
  // ALPHA = 0x41-0x5A, 0x61 - 0x7A
  // OTHER_TOKENS = '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' |
  // '^' | '_' | '`' | '|' | '~'
  TokenStream::from(quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
  })
}

#[proc_macro]
pub fn token_value(_input: TokenStream) -> TokenStream {
  // RFC 9112 section 4
  // HTAB / SP / VCHAR / obs-text
  TokenStream::from(quote! { b'\t' | b' ' | 0x21..=0x7e | 0x80..=0xff })
}

#[proc_macro]
pub fn otherwise(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as LitInt);
  let tokens = definition.base10_parse::<isize>().unwrap();
  let quotes: Vec<_> = (0..tokens).map(|x| format_ident!("_u{}", format!("{}", x))).collect();

  TokenStream::from(quote! { [ #(#quotes),*, .. ] })
}

#[proc_macro]
pub fn method(input: TokenStream) -> TokenStream {
  // HTTP: https://www.iana.org/assignments/http-methods as stated in RFC 9110 section 16.1.1
  // RTSP: RFC 7826 section 7.1

  let methods = METHODS.read().unwrap();

  let output: Vec<_> = if input.is_empty() {
    methods.iter().map(|x| quote! { string!(#x) }).collect()
  } else {
    let identifier = parse_macro_input!(input as Ident);
    methods.iter().map(|x| quote! { #identifier @ string!(#x) }).collect()
  };

  TokenStream::from(quote! { #(#output)|* })
}

#[proc_macro]
pub fn url(_input: TokenStream) -> TokenStream {
  // RFC 3986 appendix A and RFC 5234 appendix B.1
  // DIGIT = 0x30 - 0x39
  // ALPHA = 0x41-0x5A, 0x61 - 0x7A
  // OTHER_UNRESERVED_AND_RESERVED = '-' | '.' | '_' | '~' | ':' | '/' | '?' | '#'
  // | '[' | ']' | '@' | '!' | '$' | '&' | ''' | '(' | ')' | '*' | '+' | ',' | ';'
  // | '=' | '%'

  TokenStream::from(quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'-' | b'.' | b'_' | b'~' | b':' | b'/' | b'?' | b'#' | b'[' | b']' | b'@' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b'%'
  })
}
// #endregion matchers

// #region actions
#[proc_macro]
pub fn string_length(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as StringLength);

  let len = definition.string.value().len() as isize + definition.modifier;

  TokenStream::from(quote! { #len })
}

#[proc_macro]
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

#[proc_macro]
pub fn callback(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifiersWithExpr);
  let callback = definition.identifier;

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
    if #invocation != 0 {
      return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
    }
  })
}

#[proc_macro]
pub fn suspend(_input: TokenStream) -> TokenStream { TokenStream::from(quote! { SUSPEND }) }

#[proc_macro]
pub fn find_method(input: TokenStream) -> TokenStream {
  let identifier = parse_macro_input!(input as Expr);

  let methods: Vec<_> = METHODS
    .read()
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
#[proc_macro]
pub fn initial_state(_input: TokenStream) -> TokenStream {
  let initial_state = format_ident!("{}", STATES.read().unwrap()[0]);

  TokenStream::from(quote! { State::#initial_state })
}

#[proc_macro]
pub fn apply_state(_input: TokenStream) -> TokenStream {
  // Generate all the branches
  let states_arms: Vec<_> = STATES
    .read()
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

#[proc_macro]
pub fn c_match_state_string(_input: TokenStream) -> TokenStream {
  let states_to_string_arms: Vec<_> = STATES
    .read()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    match self.state {
      State::FINISH => "FINISH",
      State::ERROR => "ERROR",
      #(#states_to_string_arms),*
    }
  })
}

#[proc_macro]
pub fn c_match_error_code_string(_input: TokenStream) -> TokenStream {
  let error_to_string_arms: Vec<_> = ERRORS
    .read()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Error::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    match self.error_code {
      Error::NONE => "NONE",
      Error::UNEXPECTED_DATA => "UNEXPECTED_DATA",
      Error::UNEXPECTED_EOF => "UNEXPECTED_EOF",
      Error::CALLBACK_ERROR => "CALLBACK_ERROR",
      #(#error_to_string_arms),*
    }
  })
}

// #endregion generators

#[proc_macro]
pub fn generate_parser(_input: TokenStream) -> TokenStream {
  let methods_ref = METHODS.read().unwrap();
  let states_ref = STATES.read().unwrap();
  let errors_ref = ERRORS.read().unwrap();

  let methods: Vec<_> = methods_ref
    .iter()
    .map(|x| format_ident!("{}", x.replace("-", "_")))
    .collect();

  let states: Vec<_> = states_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let values: Vec<_> = VALUES.read().unwrap().iter().map(|x| format_ident!("{}", x)).collect();

  let errors: Vec<_> = errors_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let methods_consts: Vec<_> = methods_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const METHOD_{}: isize = {};", x.replace("-", "_"), i)).unwrap())
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
    .read()
    .unwrap()
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();

  let output = quote! {
    fn noop_internal(_parser: &mut Parser, _data: *const c_uchar, _len: usize) -> isize {
      0
    }

    /// cbindgen:ignore
    const SUSPEND: isize = #SUSPEND;

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
    #[derive(Debug)]
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

    #[repr(C)]
    #[derive(Debug)]
    pub struct Parser {
      pub owner: *mut c_void,
      pub state: State,
      pub position: usize,
      pub paused: bool,
      pub error_code: Error,
      pub error_description: *const c_uchar,
      pub error_description_len: usize,
      pub unconsumed: *const c_uchar,
      pub unconsumed_len: usize,
      #( pub #values: isize ),*,
      pub callbacks: Callbacks,
    }
  };

  TokenStream::from(output)
}

#[proc_macro]
pub fn generate_parser_initializers(_input: TokenStream) -> TokenStream {
  let values_ref = VALUES.read().unwrap();
  let values: Vec<_> = values_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let persistent_values_ref = PERSISTENT_VALUES.read().unwrap();
  let clearable_values: Vec<_> = values_ref
    .iter()
    .filter(|x| !persistent_values_ref.contains(x.as_str()))
    .map(|x| format_ident!("{}", x))
    .collect();

  TokenStream::from(quote! {
    pub fn new() -> Parser {
      Parser {
        owner: ptr::null_mut(),
        state: initial_state!(),
        position: 0,
        paused: false,
        error_code: Error::NONE,
        error_description: ptr::null(),
        error_description_len: 0,
        unconsumed: ptr::null(),
        unconsumed_len: 0,
        #( #values: 0 ),*,
        callbacks: Callbacks::new(),
      }
    }

    pub fn reset(&mut self, keep_position: bool) {
      self.state = initial_state!();
      self.paused = false;

      if !keep_position {
        self.position = 0;
      }

      self.error_code = Error::NONE;

      if self.error_description_len > 0 {
        unsafe { Vec::from_raw_parts(self.error_description as *mut c_uchar, self.error_description_len, self.error_description_len); }

        self.error_description = ptr::null();
        self.error_description_len = 0;
      }

      if self.unconsumed_len > 0 {
        unsafe { Vec::from_raw_parts(self.unconsumed as *mut c_uchar, self.unconsumed_len, self.unconsumed_len); }

        self.unconsumed = ptr::null();
        self.unconsumed_len = 0;
      }

      self.clear();
    }

    pub fn clear(&mut self) {
      #( self.#clearable_values = 0 );*
    }
  })
}
