#[macro_use]
extern crate lazy_static;

mod parsing;

use indexmap::IndexSet;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::sync::Mutex;
use syn::{parse_macro_input, parse_str, Arm, ExprMethodCall, Ident, LitByte, LitInt, LitStr};

use parsing::{Char, CharRange, Failure, Identifiers, Move, State};

lazy_static! {
  static ref STATES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref ERRORS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref VALUES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref PERSISTENT_VALUES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref SPANS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref CALLBACKS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
}

fn format_state(ident: &Ident) -> Ident {
  format_ident!("{}", ident.to_string().to_uppercase())
}

// #region definitions
#[proc_macro]
pub fn values(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::unbound);

  let mut values = VALUES.lock().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn persistent_values(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::unbound);

  let mut values = PERSISTENT_VALUES.lock().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn spans(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::unbound);

  let mut spans = SPANS.lock().unwrap();

  for span in definition.identifiers {
    spans.insert(span.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn errors(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::unbound);

  let mut errors = ERRORS.lock().unwrap();

  for error in definition.identifiers {
    errors.insert(error.to_string().to_uppercase());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn callbacks(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::unbound);

  let mut callbacks = CALLBACKS.lock().unwrap();

  for cb in definition.identifiers {
    callbacks.insert(cb.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn measure(input: TokenStream) -> TokenStream {
  let definition: State = parse_macro_input!(input as State);
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
  let definition: State = parse_macro_input!(input as State);
  let name = definition.name;
  let statements = definition.statements;

  STATES.lock().unwrap().insert(name.to_string().to_uppercase());

  TokenStream::from(quote! {
    #[inline(always)]
    fn #name (parser: &mut Parser, data: &[u8]) -> isize { #(#statements)* }
  })
}
// #endregion definitions

// #region matchers
#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream {
  let definition: Char = parse_macro_input!(input as Char);

  let quote = if let Some(identifier) = definition.identifier {
    if let Some(byte) = definition.byte {
      quote! { [#identifier @ #byte, ..] }
    } else {
      quote! { [#identifier, ..] }
    }
  } else {
    let byte = definition.byte.unwrap();
    quote! { [#byte, ..] }
  };

  TokenStream::from(quote)
}

#[proc_macro]
pub fn char_in_range(input: TokenStream) -> TokenStream {
  let definition: CharRange = parse_macro_input!(input as CharRange);
  let identifier = definition.identifier;
  let from = definition.from;
  let to = definition.to;

  TokenStream::from(quote! { #from <= *#identifier && *#identifier <= #to })
}

#[proc_macro]
pub fn string(input: TokenStream) -> TokenStream {
  let definition: LitStr = parse_macro_input!(input as LitStr);
  let bytes: Vec<LitByte> = definition
    .value()
    .bytes()
    .map(|b| LitByte::new(b, definition.span()))
    .collect();

  TokenStream::from(quote! { [#(#bytes),*, ..] })
}

#[proc_macro]
pub fn ws(_: TokenStream) -> TokenStream {
  // RFC 9110 section 5.6.3 - HTAB / SP
  TokenStream::from(quote! { [b'\t' | b' ', ..] })
}

#[proc_macro]
pub fn crlf(_: TokenStream) -> TokenStream {
  TokenStream::from(quote! { [b'\r', b'\n', ..] })
}

#[proc_macro]
pub fn digit(input: TokenStream) -> TokenStream {
  if input.is_empty() {
    TokenStream::from(quote! { [0x30..=0x39, ..] })
  } else {
    let identifier: Ident = parse_macro_input!(input as Ident);

    TokenStream::from(quote! { [#identifier @ (0x30..=0x39), ..] })
  }
}

#[proc_macro]
pub fn hex_digit(input: TokenStream) -> TokenStream {
  if input.is_empty() {
    TokenStream::from(quote! { [0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66, ..] })
  } else {
    let identifier: Ident = parse_macro_input!(input as Ident);

    TokenStream::from(quote! { [#identifier @ (0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66), ..] })
  }
}

#[proc_macro]
pub fn token(input: TokenStream) -> TokenStream {
  /*
    RFC 9110 section 5.6.2 and RFC 5234 appendix B.1
    DIGIT = 0x30 - 0x39
    ALPHA = 0x41-0x5A, 0x61 - 0x7A
    OTHER_TOKENS = '!' | '#' | '$' | '%' | '&' | '\'' | '*' | '+' | '-' | '.' | '^' | '_' | '`' | '|' | '~'
  */
  let tokens = quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
  };

  if input.is_empty() {
    TokenStream::from(quote! { [ #tokens, ..] })
  } else {
    let identifier: Ident = parse_macro_input!(input as Ident);

    TokenStream::from(quote! { [ #identifier @(#tokens) , ..] })
  }
}

#[proc_macro]
pub fn otherwise(input: TokenStream) -> TokenStream {
  let definition: LitInt = parse_macro_input!(input as LitInt);
  let tokens = definition.base10_parse::<isize>().unwrap();
  let quotes: Vec<Ident> = (0..tokens).map(|x| format_ident!("_u{}", format!("{}", x))).collect();

  TokenStream::from(quote! { [ #(#quotes),*, .. ] })
}

#[proc_macro]
pub fn method(input: TokenStream) -> TokenStream {
  /*
     HTTP: https://www.iana.org/assignments/http-methods as stated in RFC 9110 section 16.1.1
     RTSP: RFC 7826 section 7.1
  */
  let methods = [
    // HTTP
    "ACL",
    "BASELINE-CONTROL",
    "BIND",
    "CHECKIN",
    "CHECKOUT",
    "CONNECT",
    "COPY",
    "DELETE",
    "GET",
    "HEAD",
    "LABEL",
    "LINK",
    "LOCK",
    "MERGE",
    "MKACTIVITY",
    "MKCALENDAR",
    "MKCOL",
    "MKREDIRECTREF",
    "MKWORKSPACE",
    "MOVE",
    "OPTIONS",
    "ORDERPATCH",
    "PATCH",
    "POST",
    "PRI",
    "PROPFIND",
    "PROPPATCH",
    "PUT",
    "REBIND",
    "REPORT",
    "SEARCH",
    "TRACE",
    "UNBIND",
    "UNCHECKOUT",
    "UNLINK",
    "UNLOCK",
    "UPDATE",
    "UPDATEREDIRECTREF",
    "VERSION-CONTROL",
    "*",
    // RTSP
    "DESCRIBE",
    "GET_PARAMETER",
    "PAUSE",
    "PLAY",
    "PLAY_NOTIFY",
    "REDIRECT",
    "SETUP",
    "SET_PARAMETER",
    "TEARDOWN",
  ];

  let output = if input.is_empty() {
    methods.map(|x| quote! { string!(#x) })
  } else {
    let identifier: Ident = parse_macro_input!(input as Ident);
    methods.map(|x| quote! { #identifier @ string!(#x) })
  };

  TokenStream::from(quote! { #(#output)|* })
}

#[proc_macro]
pub fn url(input: TokenStream) -> TokenStream {
  /*
    RFC 3986 appendix A and RFC 5234 appendix B.1
    DIGIT = 0x30 - 0x39
    ALPHA = 0x41-0x5A, 0x61 - 0x7A
    OTHER_UNRESERVED_AND_RESERVED = '-' | '.' | '_' | '~' | ':' | '/' | '?' | '#' | '[' | ']' | '@' | '!' | '$' | '&' | ''' | '(' | ')' | '*' | '+' | ',' | ';' | '=' | '%'
  */

  let tokens = quote! {
    0x30..=0x39 |
    0x41..=0x5A |
    0x61..=0x7A |
    b'-' | b'.' | b'_' | b'~' | b':' | b'/' | b'?' | b'#' | b'[' | b']' | b'@' | b'!' | b'$' | b'&' | b'\'' | b'(' | b')' | b'*' | b'+' | b',' | b';' | b'=' | b'%'
  };

  if input.is_empty() {
    TokenStream::from(quote! { [ #tokens, ..] })
  } else {
    let identifier: Ident = parse_macro_input!(input as Ident);
    TokenStream::from(quote! { [ #identifier @ (#tokens), ..] })
  }
}
// #endregion matchers

// #region actions
#[proc_macro]
pub fn fail(input: TokenStream) -> TokenStream {
  let definition: Failure = parse_macro_input!(input as Failure);
  let error = definition.error;
  let message = definition.message;

  TokenStream::from(quote! { parser.fail_str(Error::#error, #message) })
}

#[proc_macro]
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition: Move = parse_macro_input!(input as Move);
  let state = format_state(&definition.state);
  let advance = definition.advance;

  TokenStream::from(quote! { parser.move_to(State::#state, #advance) })
}

#[proc_macro]
pub fn clear(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::one);
  let span = &definition.identifiers[0];

  TokenStream::from(quote! { parser.spans.#span.clear(); })
}

#[proc_macro]
pub fn reset(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { parser.reset(); })
}

#[proc_macro]
pub fn append(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::two);
  let span = &definition.identifiers[0];
  let value = &definition.identifiers[1];

  let callback = format_ident!("on_data_{}", &span);

  TokenStream::from(quote! {
    parser.spans.#span.push(*#value);

    #[cfg(debug_assertions)]
    if let Some(cb) = parser.callbacks.#callback {
      let action = cb(
        parser,
        std::ffi::CStr::from_bytes_with_nul(&[*#value, b'\0']).unwrap().as_ptr(),
        1
      );

      if action < 0 {
        return action;
      } else if action != 0 {
        return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
      }
    }

    #[cfg(not(debug_assertions))]
    { 0 }
  })
}

#[proc_macro]
pub fn data_slice_callback(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::minimum_two);
  let callbacks: Vec<Ident> = definition.identifiers[0..definition.identifiers.len() - 1].to_vec();
  let source = definition.identifiers.last().unwrap();

  TokenStream::from(quote! {
    #[cfg(debug_assertions)]
    {
      let callbacks: Vec<ActiveCallback> = [#(parser.callbacks.#callbacks),*]
        .iter()
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect();

      for d in #source {
        for cb in callbacks.iter() {
          let action = cb(
            parser,
            std::ffi::CStr::from_bytes_with_nul(&[*d, b'\0']).unwrap().as_ptr(),
            1,
          );

          if action < 0 {
            return action;
          } else if action != 0 {
            return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
          }
        }

        parser.position += 1;
      }
    }

    #[cfg(not(debug_assertions))]
    {
      parser.position += #source.len();
    }
  })
}

#[proc_macro]
pub fn get_span(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::one);
  let span = &definition.identifiers[0];

  TokenStream::from(quote! { parser.get_span(&parser.spans.#span) })
}

#[proc_macro]
pub fn get_value(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::one);
  let value = &definition.identifiers[0];

  TokenStream::from(quote! { parser.values.#value })
}

#[proc_macro]
pub fn set_value(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::two);
  let name = &definition.identifiers[0];
  let value = &definition.identifiers[1];

  TokenStream::from(quote! { parser.values.#name = #value as isize; })
}

#[proc_macro]
pub fn inc(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::one);
  let value = &definition.identifiers[0];

  TokenStream::from(quote! { parser.values.#value += 1; })
}

#[proc_macro]
pub fn dec(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::one);
  let value = &definition.identifiers[0];

  TokenStream::from(quote! { parser.values.#value -= 1; })
}

#[proc_macro]
pub fn callback(input: TokenStream) -> TokenStream {
  let definition: Identifiers = parse_macro_input!(input with Identifiers::one_or_two);
  let callback = &definition.identifiers[0];
  let span = definition.identifiers.get(1);

  let invocation = if let Some(span) = span {
    quote! { cb(parser, unsafe { std::ffi::CString::from_vec_unchecked(parser.spans.#span.clone()).as_c_str().as_ptr() }, parser.spans.#span.len()) }
  } else {
    quote! { cb(parser, std::ptr::null(), 0) }
  };

  TokenStream::from(quote! {
    if let Some(cb) = parser.callbacks.#callback {
      let action = #invocation;

      if action < 0 {
        return action;
      } else if action != 0 {
        return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
      }
    }
  })
}

#[proc_macro]
pub fn pause(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { isize::MIN })
}
// #endregion actions

#[proc_macro]
pub fn generate_parser(_input: TokenStream) -> TokenStream {
  let states_ref = STATES.lock().unwrap();
  let initial_state = format_ident!("{}", states_ref[0]);
  let states: Vec<Ident> = states_ref.iter().map(|x| format_ident!("{}", x)).collect();

  // Generate all the branches
  let states_arms: Vec<Arm> = states_ref
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

  let states_to_string_arms: Vec<Arm> = states_ref
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => \"{}\"", x, x)).unwrap())
    .collect();

  let values_ref = VALUES.lock().unwrap();
  let mut values: Vec<Ident> = values_ref.iter().map(|x| format_ident!("{}", x)).collect();
  values.insert(0, format_ident!("continue_without_data"));
  values.insert(0, format_ident!("mode"));

  let persistent_values_ref = PERSISTENT_VALUES.lock().unwrap();
  let clearable_values: Vec<Ident> = values_ref
    .iter()
    .filter(|x| !persistent_values_ref.contains(x.clone()))
    .map(|x| format_ident!("{}", x))
    .collect();

  let values_getters: Vec<_> = values
    .iter()
    .map(|value| {
      let getter = format_ident!("milo_get_value_{}", value);

      quote! {
        #[no_mangle]
        pub extern "C" fn #getter(parser: *mut Parser) -> isize {
          unsafe { (*parser).values.#value }
        }
      }
    })
    .collect();

  let spans: Vec<Ident> = SPANS.lock().unwrap().iter().map(|x| format_ident!("{}", x)).collect();

  let spans_getters: Vec<_> = spans
    .iter()
    .map(|span| {
      let getter = format_ident!("milo_get_span_{}", span);

      quote! {
        #[no_mangle]
        pub extern "C" fn #getter(parser: *mut Parser) -> *mut c_char {
          unsafe { CString::from_vec_unchecked((*parser).spans.#span.clone()).into_raw() }
        }
      }
    })
    .collect();

  let errors: Vec<Ident> = ERRORS.lock().unwrap().iter().map(|x| format_ident!("{}", x)).collect();

  let error_to_string_arms: Vec<Arm> = errors
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Error::{} => \"{}\"", x, x)).unwrap())
    .collect();

  let callbacks_ref = CALLBACKS.lock().unwrap();
  let mut callbacks: Vec<Ident> = callbacks_ref.iter().map(|x| format_ident!("{}", x)).collect();

  callbacks.insert(0, format_ident!("on_error"));
  callbacks.insert(0, format_ident!("on_finish"));
  callbacks.insert(0, format_ident!("after_state_change"));
  callbacks.insert(0, format_ident!("before_state_change"));

  for x in spans.iter() {
    callbacks.push(format_ident!("on_data_{}", x));
  }

  let callbacks_setters: Vec<_> = callbacks
    .iter()
    .map(|callback| {
      let setter = format_ident!("milo_set_{}", callback);

      quote! {
        #[no_mangle]
        pub extern "C" fn #setter(parser: *mut Parser, callback: Callback) {
          unsafe { (*parser).callbacks.#callback = callback };
        }
      }
    })
    .collect();

  let mut states_debug: Vec<Arm> = states
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => write!(f, \"State::{}\")", x, x)).unwrap())
    .collect();

  states_debug.insert(
    0,
    parse_str::<Arm>("State::ERROR => write!(f, \"State::ERROR\")").unwrap(),
  );

  states_debug.insert(
    0,
    parse_str::<Arm>("State::FINISH => write!(f, \"State::FINISH\")").unwrap(),
  );

  let values_debug: ExprMethodCall = parse_str::<ExprMethodCall>(&format!(
    "f.debug_struct(\"Values\"){}.finish()",
    values
      .iter()
      .map(|x| { format!(".field(\"{}\", &self.{})", x, x) })
      .collect::<Vec<String>>()
      .join("")
  ))
  .unwrap();

  let spans_debug: ExprMethodCall = parse_str::<ExprMethodCall>(&format!(
    "f.debug_struct(\"Spans\"){}.finish()",
    spans
      .iter()
      .map(|x| {
        format!(
          ".field(\"{}\", &unsafe {{ std::ffi::CString::from_vec_unchecked(self.{}.clone()) }})",
          x, x
        )
      })
      .collect::<Vec<String>>()
      .join("")
  ))
  .unwrap();

  let callbacks_debug: ExprMethodCall = parse_str::<ExprMethodCall>(&format!(
    "f.debug_struct(\"Callbacks\"){}.finish()",
    callbacks
      .iter()
      .map(|x| format!(".field(\"{}\", &self.{}.is_some())", x, x))
      .collect::<Vec<String>>()
      .join("")
  ))
  .unwrap();

  let output = quote! {
    use std::time::SystemTime;
    use std::ffi::CString;

    #[derive(Debug)]
    pub struct Parser {
      pub state: State,
      pub position: usize,
      pub values: Values,
      pub callbacks: Callbacks,
      pub spans: Spans,
      pub error_code: Error,
      pub error_description: String
    }

    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum State {
      ERROR = 0,
      FINISH,
      #(#states),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum Error {
      NONE = 0,
      UNEXPECTED_DATA,
      CALLBACK_ERROR,
      #(#errors),*
    }

    pub struct Values {
      #( pub #values: isize ),*
    }

    pub struct Spans {
      #( pub #spans: Vec<u8> ),*
    }

    type ActiveCallback = fn (&mut Parser, *const std::os::raw::c_char, usize) -> isize;
    // Do not use ActiveCallback here to ensure C headers are properly generated
    type Callback = Option<fn (&mut Parser, *const std::os::raw::c_char, usize) -> isize>;

    pub struct Callbacks {
      #( pub #callbacks: Callback),*
    }

    impl std::fmt::Display for State {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
          #(#states_debug),*
        }
      }
    }

    impl std::fmt::Debug for State {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

      fn clear(&mut self) {
        #( self.#spans.clear() );*
      }
    }

    impl Callbacks {
      fn new() -> Callbacks {
        Callbacks {
          #( #callbacks: None ),*
        }
      }

      fn clear(&mut self) {
        #( self.#callbacks = None );*
      }
    }

    impl std::fmt::Debug for Values {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #values_debug
      }
    }

    impl std::fmt::Debug for Spans {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #spans_debug
      }
    }

    impl std::fmt::Debug for Callbacks {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #callbacks_debug
      }
    }

    impl Parser {
      pub fn new() -> Parser {
        Parser {
          state: State::#initial_state,
          position: 0,
          values: Values::new(),
          spans: Spans::new(),
          callbacks: Callbacks::new(),
          error_code: Error::NONE,
          error_description: String::new(),
        }
      }

      pub fn reset(&mut self) {
        self.state = State::#initial_state;
        self.position = 0;
        self.values.clear();
        self.spans.clear();
        self.error_code = Error::NONE;
        self.error_description = String::new();
      }

      pub fn get_span(&self, span: &Vec<u8>) -> String {
        unsafe { String::from_utf8_unchecked((*span).clone()) }
      }

      fn move_to(&mut self, state: State, advance: isize) -> isize {
        #[cfg(debug_assertions)]
        {
          let fail_advance = if advance < 0 { advance } else { -advance };

          // Notify the end of the current state
          let result = if let Some(cb) = self.callbacks.after_state_change {
              cb(self, std::ptr::null(), 0)
          } else {
              0
          };

          match result {
            0 => (),
            -1 => {
                return fail_advance
            },
            _ => {
                return self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
            }
          };
        };

        // Change the state
        self.state = state;

        #[cfg(debug_assertions)]
        {
          let fail_advance = if advance < 0 { advance } else { -advance };

          let result = if let Some(cb) = self.callbacks.before_state_change {
            cb(self, std::ptr::null(), 0)
          } else {
            0
          };

          match result {
            0 => advance,
            -1 => fail_advance,
            _ => {
                return self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
            }
          }
        };

        advance
      }

      fn fail(&mut self, code: Error, reason: String) -> isize {
        self.error_code = code;
        self.error_description = reason;
        self.state = State::ERROR;

        0
      }

      fn fail_str(&mut self, code: Error, reason: &str) -> isize {
        self.fail(code, reason.into())
      }

      pub fn parse_skipping(&mut self, data: *const std::os::raw::c_char , skip: usize) -> usize {
        let mut consumed: usize = 0;
        let mut current = unsafe { std::ffi::CStr::from_ptr(data) }.to_bytes();

        // Notify the initial status - Note this invocation is not pauseable
        #[cfg(debug_assertions)]
        if self.position == 0 {
          if let Some(cb) = self.callbacks.before_state_change {
            if cb(self, std::ptr::null(), 0) > 0 {
              self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
            }
          }
        }

        if skip > 0 {
          current = &current[skip..];
        }

        #[cfg(debug_assertions)]
        let mut last = SystemTime::now();

        while current.len() > 0 || self.values.continue_without_data == 1 {
          self.values.continue_without_data = 0;

          // Since states might advance position manually, we have to explicitly track it
          let initial_position = self.position;

          if let State::FINISH = self.state {
            self.fail_str(Error::UNEXPECTED_DATA, "unexpected data");
            continue;
          }

          let result = match self.state {
            State::FINISH => 0,
            State::ERROR => 0,
            #(#states_arms),*,
          };

          match &self.state {
            State::FINISH => {
              if let Some(cb) = self.callbacks.on_finish {
                cb(self, std::ptr::null(), 0);
              }
            },
            State::ERROR => {
              if let Some(cb) = self.callbacks.on_error {
                let error = self.error_description.as_str();
                cb(self, std::ffi::CString::new(error).unwrap().as_c_str().as_ptr(), error.len());
              }

              return consumed;
            },
            _ => {}
          }

          /*
            Negative return values mean to consume N bytes and then pause.
            Returning MIN instruct to pause without consuming any byte.
          */
          if result < 0 {
            if result == isize::MIN {
              return consumed;
            }

            let advance = -result as usize;
            self.position += advance;
            consumed += advance;
            return consumed;
          }

          let advance = result as usize;
          self.position += advance;

          let difference = self.position - initial_position;
          consumed += difference;
          current = &current[difference..];

          #[cfg(all(debug_assertions, feature = "milo_debug_loop"))]
          {
            let duration = SystemTime::now().duration_since(last).unwrap().as_nanos();

            if duration > 10 {
              println!("[milo::debug] loop iteration (ending in state {}) completed in {} ns", self.state, duration);
            }

            last = SystemTime::now();
          }
        }

        consumed
      }

      pub fn parse(&mut self, data: *const std::os::raw::c_char) -> usize {
        self.parse_skipping(data, 0)
      }
    }

    #[no_mangle]
    pub extern "C" fn milo_free_string(s: *mut c_char) {
      unsafe {
        if s.is_null() {
          return;
        }

        let _ = CString::from_raw(s);
      }
    }

    #[no_mangle]
    pub extern "C" fn milo_init() -> *mut Parser {
      Box::into_raw(Box::new(Parser::new()))
    }

    #[no_mangle]
    pub extern "C" fn milo_free(ptr: *mut Parser) {
      if ptr.is_null() {
        return;
      }

      unsafe {
        let _ =  Box::from_raw(ptr);
      }
    }

    #[no_mangle]
    pub extern "C" fn milo_parse(parser: *mut Parser, data: *const c_char) -> usize {
      unsafe { parser.as_mut().unwrap().parse(data) }
    }

    #[no_mangle]
    pub extern "C" fn milo_get_state(parser: *mut Parser) -> u8 {
      unsafe { parser.as_mut().unwrap().state as u8 }
    }

    #[no_mangle]
    pub extern "C" fn milo_get_state_string(parser: *mut Parser) -> *mut c_char {
      let string = match unsafe { (*parser).state } {
        State::FINISH => "FINISH",
        State::ERROR => "ERROR",
        #(#states_to_string_arms),*
      };

      std::ffi::CString::new(string).unwrap().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn milo_get_position(parser: *mut Parser) -> usize {
      unsafe { (*parser).position }
    }

    #(#values_getters)*

    #(#spans_getters)*

    #[no_mangle]
    pub extern "C" fn milo_get_error_code(parser: *mut Parser) -> usize {
      unsafe { (*parser).position }
    }

    #[no_mangle]
    pub extern "C" fn milo_get_error_code_string(parser: *mut Parser) -> *mut c_char {
      let string = match unsafe { (*parser).error_code } {
        Error::NONE => "NONE",
        Error::UNEXPECTED_DATA => "UNEXPECTED_DATA`",
        Error::CALLBACK_ERROR => "CALLBACK_ERROR",

        #(#error_to_string_arms),*
      };

      std::ffi::CString::new(string).unwrap().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn milo_get_error_code_description(parser: *mut Parser) -> *mut c_char {
      unsafe { std::ffi::CString::new((*parser).error_description.clone()).unwrap().into_raw() }
    }

    #(#callbacks_setters)*
  };

  TokenStream::from(output)
}
