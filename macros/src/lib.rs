#[macro_use]
extern crate lazy_static;

mod parsing;

use indexmap::IndexSet;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::ffi::c_uchar;
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;
use syn::{
  parse_macro_input, parse_str, Arm, Expr, ExprMethodCall, Ident, ItemConst, LitByte, LitChar, LitInt, LitStr,
};

use parsing::{Failure, Identifiers, IdentifiersWithExpr, State, StringLength};

const SUSPEND: isize = isize::MIN;

lazy_static! {
  static ref METHODS: Mutex<Vec<String>> = {
    let mut absolute_path = Path::new(file!()).parent().unwrap().to_path_buf();
    absolute_path.push("methods.yml");
    let f = File::open(absolute_path.to_str().unwrap()).unwrap();
    let methods = serde_yaml::from_reader(f).unwrap();

    Mutex::new(methods)
  };
  static ref STATES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref ERRORS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref VALUES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref PERSISTENT_VALUES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref USER_WRITABLE_VALUES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref SPANS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref CALLBACKS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
}

fn format_state(ident: &Ident) -> Ident {
  format_ident!("{}", ident.to_string().to_uppercase())
}

// #region definitions
#[proc_macro]
pub fn values(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut values = VALUES.lock().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn user_writable_values(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut values = USER_WRITABLE_VALUES.lock().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn persistent_values(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut values = PERSISTENT_VALUES.lock().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn spans(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut spans = SPANS.lock().unwrap();

  for span in definition.identifiers {
    spans.insert(span.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn errors(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut errors = ERRORS.lock().unwrap();

  for error in definition.identifiers {
    errors.insert(error.to_string().to_uppercase());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn callbacks(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let mut callbacks = CALLBACKS.lock().unwrap();

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
  let statements = definition.statements;

  STATES.lock().unwrap().insert(name.to_string().to_uppercase());

  TokenStream::from(quote! {
    #[inline(always)]
    fn #name (parser: &mut Parser, data: &[c_uchar]) -> isize {
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
pub fn digit(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { 0x30..=0x39 })
}

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
pub fn crlf(_: TokenStream) -> TokenStream {
  TokenStream::from(quote! { [b'\r', b'\n', ..] })
}

#[proc_macro]
pub fn double_crlf(_: TokenStream) -> TokenStream {
  TokenStream::from(quote! { [b'\r', b'\n', b'\r', b'\n', ..] })
}

#[proc_macro]
pub fn token(_input: TokenStream) -> TokenStream {
  /*
    RFC 9110 section 5.6.2 and RFC 5234 appendix B.1
    DIGIT = 0x30 - 0x39
    ALPHA = 0x41-0x5A, 0x61 - 0x7A
    OTHER_TOKENS = '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' | '^' | '_' | '`' | '|' | '~'
  */
  TokenStream::from(quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
  })
}

#[proc_macro]
pub fn token_value(_input: TokenStream) -> TokenStream {
  /*
    RFC 9112 section 4
    HTAB / SP / VCHAR / obs-text
  */
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
  /*
     HTTP: https://www.iana.org/assignments/http-methods as stated in RFC 9110 section 16.1.1
     RTSP: RFC 7826 section 7.1
  */

  let methods = METHODS.lock().unwrap();

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
  /*
    RFC 3986 appendix A and RFC 5234 appendix B.1
    DIGIT = 0x30 - 0x39
    ALPHA = 0x41-0x5A, 0x61 - 0x7A
    OTHER_UNRESERVED_AND_RESERVED = '-' | '.' | '_' | '~' | ':' | '/' | '?' | '#' | '[' | ']' | '@' | '!' | '$' | '&' | ''' | '(' | ')' | '*' | '+' | ',' | ';' | '=' | '%'
  */

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
pub fn suspend(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { SUSPEND })
}

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
          "[{}, ..] => {{ parser.values.is_connect_request = 1; {} }}",
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
  let initial_state = format_ident!("{}", STATES.lock().unwrap()[0]);

  TokenStream::from(quote! { State::#initial_state })
}

#[proc_macro]
pub fn apply_state(_input: TokenStream) -> TokenStream {
  // Generate all the branches
  let states_arms: Vec<_> = STATES
    .lock()
    .unwrap()
    .iter()
    .map(|x| {
      parse_str::<Arm>(&format!(
        "State::{} => {}(self, current)",
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
    .lock()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    match unsafe { (*parser).state } {
      State::FINISH => "FINISH",
      State::ERROR => "ERROR",
      #(#states_to_string_arms),*
    }
  })
}

#[proc_macro]
pub fn c_match_error_code_string(_input: TokenStream) -> TokenStream {
  let error_to_string_arms: Vec<_> = ERRORS
    .lock()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Error::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    match unsafe { (*parser).error_code } {
      Error::NONE => "NONE",
      Error::UNEXPECTED_DATA => "UNEXPECTED_DATA",
      Error::UNEXPECTED_EOF => "UNEXPECTED_EOF",
      Error::CALLBACK_ERROR => "CALLBACK_ERROR",
      #(#error_to_string_arms),*
    }
  })
}

#[proc_macro]
pub fn c_values_getters(_input: TokenStream) -> TokenStream {
  let values_getters: Vec<_> = VALUES
    .lock()
    .unwrap()
    .iter()
    .map(|value| {
      let getter = format_ident!("get_{}", value);
      let key = format_ident!("{}", value);

      quote! {
        #[no_mangle]
        pub extern "C" fn #getter(parser: *mut Parser) -> isize {
          unsafe { (*parser).values.#key }
        }
      }
    })
    .collect();

  TokenStream::from(quote! {
    #(#values_getters)*
  })
}

#[proc_macro]
pub fn c_values_setters(_input: TokenStream) -> TokenStream {
  let values_setters: Vec<_> = USER_WRITABLE_VALUES
    .lock()
    .unwrap()
    .iter()
    .map(|value| {
      let setter = format_ident!("set_{}", value);
      let key = format_ident!("{}", value);

      quote! {
        #[no_mangle]
        pub extern "C" fn #setter(parser: *mut Parser, value: isize) {
          unsafe { (*parser).values.#key = value; }
        }
      }
    })
    .collect();

  TokenStream::from(quote! {
    #(#values_setters)*
  })
}

#[proc_macro]
pub fn c_spans_getters(_input: TokenStream) -> TokenStream {
  let spans_getters: Vec<_> = SPANS
    .lock()
    .unwrap()
    .iter()
    .map(|span| {
      let getter = format_ident!("get_{}_string", span);
      let key = format_ident!("{}", span);

      quote! {
        #[no_mangle]
        pub extern "C" fn #getter(parser: *mut Parser) -> *const c_uchar {
          unsafe { CString::from_vec_unchecked((*parser).spans.#key.clone()).into_raw() as *const c_uchar }
        }
      }
    })
    .collect();

  TokenStream::from(quote! {
    #(#spans_getters)*
  })
}

#[proc_macro]
pub fn c_callbacks_setters(_input: TokenStream) -> TokenStream {
  let callbacks_setters: Vec<_> = CALLBACKS
    .lock()
    .unwrap()
    .iter()
    .map(|callback| {
      let setter = format_ident!("set_{}", callback);
      let key = format_ident!("{}", callback);

      quote! {
        #[no_mangle]
        pub extern "C" fn #setter(parser: *mut Parser, callback: Callback) {
          unsafe { (*parser).callbacks.#key = callback };
        }
      }
    })
    .collect();

  TokenStream::from(quote! {
    #(#callbacks_setters)*
  })
}
// #endregion generators

#[proc_macro]
pub fn generate_parser(_input: TokenStream) -> TokenStream {
  let methods_ref = METHODS.lock().unwrap();

  let methods: Vec<_> = methods_ref
    .iter()
    .map(|x| format_ident!("{}", x.replace("-", "_")))
    .collect();

  let methods_consts: Vec<_> = methods_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const METHOD_{}: isize = {};", x.replace("-", "_"), i)).unwrap())
    .collect();

  let states_ref = STATES.lock().unwrap();
  let states: Vec<_> = states_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let states_consts: Vec<_> = states_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const STATES_{}: isize = {};", x, i)).unwrap())
    .collect();

  let values_ref = VALUES.lock().unwrap();
  let values: Vec<_> = values_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let persistent_values_ref = PERSISTENT_VALUES.lock().unwrap();
  let clearable_values: Vec<_> = values_ref
    .iter()
    .filter(|x| !persistent_values_ref.contains(x.clone()))
    .map(|x| format_ident!("{}", x))
    .collect();

  let spans: Vec<_> = SPANS.lock().unwrap().iter().map(|x| format_ident!("{}", x)).collect();

  let errors_ref = ERRORS.lock().unwrap();

  let errors: Vec<_> = errors_ref.iter().map(|x| format_ident!("{}", x)).collect();

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

  let states_debug: Vec<_> = states
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => write!(f, \"State::{}\")", x, x)).unwrap())
    .collect();

  let values_debug = parse_str::<ExprMethodCall>(&format!(
    "f.debug_struct(\"Values\"){}.finish()",
    values
      .iter()
      .map(|x| { format!(".field(\"{}\", &self.{})", x, x) })
      .collect::<Vec<String>>()
      .join("")
  ))
  .unwrap();

  let spans_debug = parse_str::<ExprMethodCall>(&format!(
    "f.debug_struct(\"Spans\"){}.finish()",
    spans
      .iter()
      .map(|x| {
        format!(
          ".field(\"{}\", &unsafe {{ str::from_utf8_unchecked(&self.{}[..]) }})",
          x, x
        )
      })
      .collect::<Vec<String>>()
      .join("")
  ))
  .unwrap();

  let callbacks_debug = parse_str::<ExprMethodCall>(&format!("f.debug_struct(\"Callbacks\").finish()",)).unwrap();

  let output = quote! {
    fn noop_internal(_parser: &mut Parser, _data: *const c_uchar, _len: usize) -> isize {
      0
    }

    const SUSPEND: isize = #SUSPEND;

    #(#errors_consts)*

    #(#methods_consts)*

    #(#states_consts)*

    #[derive(Debug)]
    pub struct Parser {
      pub owner: Option<*mut c_void>,
      pub state: State,
      pub paused: bool,
      pub position: usize,
      pub values: Values,
      pub callbacks: Callbacks,
      pub spans: Spans,
      pub error_code: Error,
      pub error_description: Vec<c_uchar>
    }

    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum Method {
      #(#methods),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum State {
      #(#states),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum Error {
      #(#errors),*
    }

    pub struct Values {
      #( pub #values: isize ),*
    }

    pub struct Spans {
      #( pub #spans: Vec<c_uchar> ),*
    }
    type Callback = fn (&mut Parser, *const c_uchar, usize) -> isize;

    pub struct Callbacks {
      #( pub #callbacks: Callback),*
    }

    impl Display for State {
      fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match *self {
          #(#states_debug),*
        }
      }
    }

    impl Debug for State {
      fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match *self {
          #(#states_debug),*
        }
      }
    }

    impl Values {
      fn new() -> Values {
        Values {
          #( #values: 0 ),*
        }
      }

      fn clear(&mut self) {
        #( self.#clearable_values = 0 );*
      }
    }

    impl Spans {
      fn new() -> Spans {
        Spans {
          #( #spans: vec![] ),*
        }
      }
    }

    impl Callbacks {
      fn new() -> Callbacks {
        Callbacks {
          #( #callbacks: noop_internal ),*
        }
      }
    }

    impl Debug for Values {
      fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #values_debug
      }
    }

    impl Debug for Spans {
      fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #spans_debug
      }
    }

    impl Debug for Callbacks {
      fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        #callbacks_debug
      }
    }
  };

  TokenStream::from(output)
}
