#[macro_use]
extern crate lazy_static;

mod parsing;

use indexmap::IndexSet;
use proc_macro::TokenStream;
use quote::{ format_ident, quote };
use std::sync::Mutex;
use syn::{ parse_macro_input, parse_str, Arm, ExprMethodCall, Ident, LitByte, LitInt, LitStr };

use parsing::{ Failure, Char, CharRange, Definition, State };

lazy_static! {
  static ref STATES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref ERRORS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref VALUES: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref SPANS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
  static ref CALLBACKS: Mutex<IndexSet<String>> = Mutex::new(IndexSet::new());
}

fn format_state(ident: &Ident) -> Ident {
  format_ident!("{}", ident.to_string().to_uppercase())
}

fn invoke_callback(
  callback: &Ident,
  span: Option<&Ident>,
  next: Option<Ident>,
  advance: isize
) -> proc_macro2::TokenStream {
  let cb = if let Some(span) = span {
    quote! { cb(parser, unsafe { std::ffi::CString::from_vec_unchecked(parser.spans.#span.clone()).as_c_str().as_ptr() }, parser.spans.#span.len()) }
  } else {
    quote! { cb(parser, std::ptr::null(), 0) }
  };

  let ret = if let Some(raw) = next {
    let state = format_state(&raw);

    quote! {
        match result {
            0 => parser.move_to(State::#state, #advance),
            -1 => parser.move_to(State::#state, -#advance),
            _ => parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error."),
        }
    }
  } else {
    quote! {
        match result {
            0 => #advance,
            -1 => -1,
            _ => parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error."),
        }
    }
  };

  quote! {
      let result = if let Some(cb) = parser.callbacks.#callback {
          #cb
      } else {
          0
      };

      #ret
  }
}

#[proc_macro]
pub fn values(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::identifiers_only);

  let mut values = VALUES.lock().unwrap();

  for value in definition.identifiers {
    values.insert(value.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn spans(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::identifiers_only);

  let mut spans = SPANS.lock().unwrap();

  for span in definition.identifiers {
    spans.insert(span.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn errors(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::identifiers_only);

  let mut errors = ERRORS.lock().unwrap();

  for error in definition.identifiers {
    errors.insert(error.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn callbacks(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::identifiers_only);

  let mut callbacks = CALLBACKS.lock().unwrap();

  for cb in definition.identifiers {
    callbacks.insert(cb.to_string());
  }

  TokenStream::new()
}

#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream {
  let definition: State = parse_macro_input!(input as State);
  let name = definition.name;
  let statements = definition.statements;

  STATES.lock().unwrap().insert(name.to_string().to_uppercase());

  TokenStream::from(
    quote! {
      #[inline(always)]
      fn #name (parser: &mut Parser, data: &[u8]) -> isize { #(#statements)* }
  }
  )
}

#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream {
  let definition: Char = parse_macro_input!(input as Char);

  TokenStream::from(match definition {
    Char::MATCH(b) => quote! { [#b, ..] },
    Char::ASSIGNMENT(b) => quote! { [#b, ..] },
  })
}

#[proc_macro]
pub fn char_in_range(input: TokenStream) -> TokenStream {
  let definition: CharRange = parse_macro_input!(input as CharRange);
  let identifier = definition.identifier;
  let from = definition.from;
  let to = definition.to;

  TokenStream::from(quote! {
      #from <= *#identifier && *#identifier <= #to
  })
}

#[proc_macro]
pub fn string(input: TokenStream) -> TokenStream {
  let definition: LitStr = parse_macro_input!(input as LitStr);
  let bytes: Vec<LitByte> = definition
    .value()
    .bytes()
    .map(|b| LitByte::new(b, definition.span()))
    .collect();

  TokenStream::from(quote! {
      [#(#bytes),*, ..]
  })
}

#[proc_macro]
pub fn ws(_: TokenStream) -> TokenStream {
  TokenStream::from(quote! {
      // RFC 9110 section 5.6.3 - HTAB / SP
      [b'\t' | b' ', ..]
  })
}

#[proc_macro]
pub fn crlf(_: TokenStream) -> TokenStream {
  TokenStream::from(quote! {
      [b'\r', b'\n', ..]
  })
}

#[proc_macro]
pub fn digit(input: TokenStream) -> TokenStream {
  if input.is_empty() {
    TokenStream::from(quote! {
        [0x30..=0x39, ..]
    })
  } else {
    let identifier: Ident = parse_macro_input!(input as Ident);

    TokenStream::from(quote! {
        [#identifier @ (0x30..=0x39), ..]
    })
  }
}

#[proc_macro]
pub fn hex_digit(input: TokenStream) -> TokenStream {
  if input.is_empty() {
    TokenStream::from(quote! {
        [0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66, ..]
    })
  } else {
    let identifier: Ident = parse_macro_input!(input as Ident);

    TokenStream::from(quote! {
        [#identifier @ (0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66), ..]
    })
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
  let tokens =
    quote! {
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

  let tokens =
    quote! {
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

#[proc_macro]
pub fn fail(input: TokenStream) -> TokenStream {
  let definition: Failure = parse_macro_input!(input as Failure);
  let ident = definition.ident;
  let message = definition.message;

  TokenStream::from(quote! { parser.fail_str(Error::#ident, #message) })
}

#[proc_macro]
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::one);
  let state = format_state(&definition.identifiers[0]);
  let advance = definition.advance;

  TokenStream::from(quote! {
      parser.move_to(State::#state, #advance)
  })
}

#[proc_macro]
pub fn clear(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::one);
  let span = &definition.identifiers[0];
  let advance = definition.advance;

  let output = if let Some(raw) = definition.next {
    let state = format_state(&raw);

    quote! {
        {
            parser.spans.#span.clear();
            parser.move_to(State::#state, #advance)
        }
    }
  } else {
    quote! {
        {
            parser.spans.#span.clear();
            #advance
        }
    }
  };

  TokenStream::from(output)
}

#[proc_macro]
pub fn reset(input: TokenStream) -> TokenStream {
  let mut advance = 0;

  // Now get the advance
  if !input.is_empty() {
    let lit: LitInt = parse_macro_input!(input as LitInt);
    advance = lit.base10_parse::<isize>().unwrap();
  }

  TokenStream::from(quote! {
      {
          parser.reset();
          #advance
      }
  })
}

#[proc_macro]
pub fn append(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::two);
  let span = &definition.identifiers[0];
  let value = &definition.identifiers[1];

  let callback = format_ident!("on_data_{}", &span);
  let invocation = invoke_callback(&callback, Some(&span), definition.next, definition.advance);

  TokenStream::from(quote! {
      {
          parser.spans.#span.push(*#value);
          #invocation
      }
  })
}

#[proc_macro]
pub fn get_span(input: TokenStream) -> TokenStream {
  let definition: Ident = parse_macro_input!(input as Ident);

  TokenStream::from(quote! { parser.get_span(&parser.spans.#definition) })
}

#[proc_macro]
pub fn get_value(input: TokenStream) -> TokenStream {
  let definition: Ident = parse_macro_input!(input as Ident);

  TokenStream::from(quote! { parser.values.#definition })
}

#[proc_macro]
pub fn set_value(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::two);
  let name = &definition.identifiers[0];
  let value = &definition.identifiers[1];
  let advance = definition.advance;

  let output = if let Some(raw) = definition.next {
    let state = format_state(&raw);

    quote! {
        {
            parser.values.#name = #value as isize;
            parser.move_to(State::#state, #advance)
        }
    }
  } else {
    quote! {
        {
            parser.values.#name = #value as isize;
            #advance
        }
    }
  };

  TokenStream::from(output)
}

#[proc_macro]
pub fn inc(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::one);
  let name = &definition.identifiers[0];
  let advance = definition.advance;

  let output = if let Some(raw) = definition.next {
    let state = format_state(&raw);

    quote! {
        {
            parser.values.#name ++;
            parser.move_to(State::#state, #advance)
        }
    }
  } else {
    quote! {
        {
            parser.values.#name ++;
            #advance
        }
    }
  };

  TokenStream::from(output)
}

#[proc_macro]
pub fn dec(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::one);
  let name = &definition.identifiers[0];
  let advance = definition.advance;

  let output = if let Some(raw) = definition.next {
    let state = format_state(&raw);

    quote! {
        {
            parser.values.#name --;
            parser.move_to(State::#state, #advance)
        }
    }
  } else {
    quote! {
        {
            parser.values.#name --;
            #advance
        }
    }
  };

  TokenStream::from(output)
}

#[proc_macro]
pub fn callback(input: TokenStream) -> TokenStream {
  let definition: Definition = parse_macro_input!(input with Definition::one_or_two);
  let callback = &definition.identifiers[0];
  let span = definition.identifiers.get(1);

  let invocation = invoke_callback(&callback, span, definition.next, definition.advance);

  TokenStream::from(quote! {
      {
          #invocation
      }
  })
}

#[proc_macro]
pub fn pause(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { isize::MIN })
}

#[proc_macro]
pub fn generate_parser(_input: TokenStream) -> TokenStream {
  let states_ref = STATES.lock().unwrap();
  let initial_state = format_ident!("{}", states_ref[0]);
  let mut states: Vec<Ident> = states_ref
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();
  states.insert(0, format_ident!("FINISH"));
  states.insert(0, format_ident!("ERROR"));

  // Generate all the branches
  let states_arms: Vec<Arm> = states
    .iter()
    .filter(|x| {
      let name = x.to_string();
      name != "ERROR" && name != "FINISH"
    })
    .map(|x| { parse_str::<Arm>(&format!("State::{} => {}(self, current)", x, x.to_string().to_lowercase())).unwrap() })
    .collect();

  let states_move_before_arms: Vec<Arm> = states
    .iter()
    .map(|x| {
      parse_str::<Arm>(&format!("State::{} => self.callbacks.before_{}", x, x.to_string().to_lowercase())).unwrap()
    })
    .collect();

  let states_move_after_arms: Vec<Arm> = states
    .iter()
    .map(|x| {
      parse_str::<Arm>(&format!("State::{} => self.callbacks.after_{}", x, x.to_string().to_lowercase())).unwrap()
    })
    .collect();

  let values_ref = VALUES.lock().unwrap();
  let mut values: Vec<Ident> = values_ref
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();
  values.insert(0, format_ident!("parse_empty_data"));
  values.insert(0, format_ident!("error_code"));

  let values_clearable: Vec<Ident> = values.clone();
  values.insert(0, format_ident!("mode"));

  let spans_ref = SPANS.lock().unwrap();
  let mut spans: Vec<Ident> = spans_ref
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();
  spans.insert(0, format_ident!("error_reason"));

  let spans_clearable = spans.clone();
  spans.insert(0, format_ident!("debug"));

  let errors_ref = ERRORS.lock().unwrap();
  let mut errors: Vec<Ident> = errors_ref
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();

  errors.insert(0, format_ident!("CALLBACK_ERROR"));
  errors.insert(0, format_ident!("UNEXPECTED_DATA"));

  let callbacks_ref = CALLBACKS.lock().unwrap();
  let mut callbacks: Vec<Ident> = callbacks_ref
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();

  callbacks.insert(0, format_ident!("on_error"));
  callbacks.insert(0, format_ident!("on_finish"));

  callbacks.push(format_ident!("before_error"));
  callbacks.push(format_ident!("after_error"));
  callbacks.push(format_ident!("before_finish"));
  callbacks.push(format_ident!("after_finish"));

  for x in states_ref.iter() {
    let cb = x.to_string().to_lowercase();
    callbacks.push(format_ident!("before_{}", cb));
    callbacks.push(format_ident!("after_{}", cb));
  }

  for x in spans_ref.iter() {
    callbacks.push(format_ident!("on_data_{}", x));
  }

  let states_debug: Vec<Arm> = states
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => write!(f, \"State::{}\")", x, x)).unwrap())
    .collect();

  let values_debug: ExprMethodCall = parse_str::<ExprMethodCall>(
    &format!(
      "f.debug_struct(\"Values\"){}.finish()",
      values_ref
        .iter()
        .map(|x| { format!(".field(\"{}\", &self.{})", x, x) })
        .collect::<Vec<String>>()
        .join("")
    )
  ).unwrap();

  let spans_debug: ExprMethodCall = parse_str::<ExprMethodCall>(
    &format!(
      "f.debug_struct(\"Spans\"){}.finish()",
      spans_ref
        .iter()
        .map(|x| {
          format!(".field(\"{}\", &unsafe {{ std::ffi::CString::from_vec_unchecked(self.{}.clone()) }})", x, x)
        })
        .collect::<Vec<String>>()
        .join("")
    )
  ).unwrap();

  let callbacks_debug: ExprMethodCall = parse_str::<ExprMethodCall>(
    &format!(
      "f.debug_struct(\"Callbacks\"){}.finish()",
      callbacks
        .iter()
        .map(|x| format!(".field(\"{}\", &self.{}.is_some())", x, x))
        .collect::<Vec<String>>()
        .join("")
    )
  ).unwrap();

  let output =
    quote! {
      pub enum State {
        #(#states),*
      }

      pub enum Error {
        #(#errors),*
      }

      pub struct Values {
          #( pub #values: isize ),*
      }

      pub struct Spans {
          #( pub #spans: Vec<u8> ),*
      }

      pub struct Callbacks {
          #( pub #callbacks: Option<fn (&mut Parser, *const std::os::raw::c_char, usize) -> isize>),*
      }

      #[derive(Debug)]
      pub struct Parser {
          pub state: State,
          pub position: usize,
          pub values: Values,
          pub callbacks: Callbacks,
          pub spans: Spans,
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
              #( self.#values_clearable = 0 );*
          }
      }

      impl Spans {
          fn new() -> Spans {
              Spans {
                  #( #spans: vec![] ),*
              }
          }

          fn clear(&mut self) {
              #( self.#spans_clearable.clear() );*
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
              }
          }

          pub fn reset(&mut self) {
              self.state = State::#initial_state;
              self.position = 0;
              self.values.clear();
              self.spans.clear();
          }

          pub fn get_span(&self, span: &Vec<u8>) -> String {
              unsafe { String::from_utf8_unchecked((*span).clone()) }
          }

          fn move_to(&mut self, state: State, advance: isize) -> isize {
              let fail_advance = if advance < 0 { advance } else { -advance };

              // Notify the end of the current state
              let option_callback = match self.state {
                  #(#states_move_after_arms),*,
              };

              let result = if let Some(cb) = option_callback {
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

              // Change the state
              self.state = state;

              // Notify the start of the current state
              let option_callback = match self.state {
                  #(#states_move_before_arms),*,
              };

              let result = if let Some(cb) = option_callback {
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
          }

          fn fail(&mut self, code: Error, reason: String) -> isize {
              self.values.error_code = code as isize;
              self.spans.error_reason = reason.as_bytes().into();
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
              if self.position == 0 {
                  let option_callback = match self.state {
                      #(#states_move_before_arms),*,
                  };

                  if let Some(cb) = option_callback {
                      if cb(self, std::ptr::null(), 0) > 0 {
                          self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
                      }
                  }
              }

              if skip > 0 {
                  current = &current[skip..];
              }

              while current.len() > 0 || self.values.parse_empty_data == 1 {
                  self.values.parse_empty_data = 0;

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
                              let error = self.get_span(&self.spans.error_reason);
                              cb(self, std::ffi::CString::new(error.as_str()).unwrap().as_c_str().as_ptr(), error.len());
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
                  consumed += advance;
                  current = &current[advance..];
              }

              consumed
          }

          pub fn parse(&mut self, data: *const std::os::raw::c_char) -> usize {
              self.parse_skipping(data, 0)
          }
      }
  };

  TokenStream::from(output)
}
