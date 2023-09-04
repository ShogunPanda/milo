#[macro_use]
extern crate lazy_static;

mod parsing;

use indexmap::IndexSet;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::fs::File;
use std::path::Path;
use std::sync::Mutex;
use syn::{parse_macro_input, parse_str, Arm, ExprMethodCall, Ident, ItemConst, LitByte, LitInt, LitStr};

use parsing::{Char, CharRange, Failure, Identifiers, Move, State};

const RESERVED_NEGATIVE_ADVANCES: isize = isize::MIN + 10;
const SUSPEND: isize = RESERVED_NEGATIVE_ADVANCES - 1;
const PAUSE: isize = RESERVED_NEGATIVE_ADVANCES - 2;

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
    fn #name (parser: &mut Parser, data: &[c_uchar]) -> isize { #(#statements)* }
  })
}
// #endregion definitions

// #region matchers
#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Char);

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
  let definition = parse_macro_input!(input as CharRange);
  let identifier = definition.identifier;
  let from = definition.from;
  let to = definition.to;

  TokenStream::from(quote! { #from <= *#identifier && *#identifier <= #to })
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
    let identifier = parse_macro_input!(input as Ident);

    TokenStream::from(quote! { [#identifier @ (0x30..=0x39), ..] })
  }
}

#[proc_macro]
pub fn hex_digit(input: TokenStream) -> TokenStream {
  if input.is_empty() {
    TokenStream::from(quote! { [0x30..=0x39 | 0x41..=0x46 | 0x61..=0x66, ..] })
  } else {
    let identifier = parse_macro_input!(input as Ident);

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
    let identifier = parse_macro_input!(input as Ident);

    TokenStream::from(quote! { [ #identifier @(#tokens) , ..] })
  }
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
    let identifier = parse_macro_input!(input as Ident);
    TokenStream::from(quote! { [ #identifier @ (#tokens), ..] })
  }
}
// #endregion matchers

// #region actions
#[proc_macro]
pub fn fail(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Failure);
  let error = definition.error;
  let message = definition.message;

  TokenStream::from(quote! { parser.fail_str(Error::#error, #message) })
}

#[proc_macro]
pub fn move_to(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as Move);
  let state = format_state(&definition.state);
  let advance = definition.advance;

  TokenStream::from(quote! { parser.move_to(State::#state, #advance) })
}

#[proc_macro]
pub fn clear(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::one);
  let span = &definition.identifiers[0];

  TokenStream::from(quote! { parser.spans.#span.clear(); })
}

#[proc_macro]
pub fn reset(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { parser.reset(false); })
}

#[proc_macro]
pub fn append(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::two);
  let span = &definition.identifiers[0];
  let value = &definition.identifiers[1];

  let callback = format_ident!("on_data_{}", &span);

  TokenStream::from(quote! {
    parser.spans.#span.push(*#value);

    #[cfg(debug_assertions)]
    {
      let action = (parser.callbacks.#callback)(parser, #value, 1);

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
pub fn append_lowercase(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::two);
  let span = &definition.identifiers[0];
  let value = &definition.identifiers[1];

  let callback = format_ident!("on_data_{}", &span);

  TokenStream::from(quote! {
    parser.spans.#span.push((*#value).to_ascii_lowercase());

    #[cfg(debug_assertions)]
    {
      let action = (parser.callbacks.#callback)(parser, #value, 1);

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
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let second_last_index = definition.identifiers.len() - 2;
  let callbacks: &[Ident] = &definition.identifiers[0..second_last_index];
  let source = definition.identifiers.get(second_last_index).unwrap();
  let len = definition.identifiers.last().unwrap();

  TokenStream::from(quote! {
    {
      #(
        let action = (parser.callbacks.#callbacks)(parser, #source.as_ptr(), #len);

        if action < 0 {
          return action;
        } else if action != 0 {
          return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
        }
      )*

      parser.position += #len;
    }
  })
}

#[proc_macro]
pub fn get_span(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::one);
  let span = &definition.identifiers[0];

  TokenStream::from(quote! {
    unsafe {
      let slice = parser.spans.#span.clone();
      String::from_utf8_unchecked(slice)
    }
  })
}

#[proc_macro]
pub fn get_value(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::one);
  let value = &definition.identifiers[0];

  TokenStream::from(quote! { parser.values.#value })
}

#[proc_macro]
pub fn set_value(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::two);
  let name = &definition.identifiers[0];
  let value = &definition.identifiers[1];

  TokenStream::from(quote! { parser.values.#name = #value as isize; })
}

#[proc_macro]
pub fn inc(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::one);
  let value = &definition.identifiers[0];

  TokenStream::from(quote! { parser.values.#value += 1; })
}

#[proc_macro]
pub fn dec(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::one);
  let value = &definition.identifiers[0];

  TokenStream::from(quote! { parser.values.#value -= 1; })
}

#[proc_macro]
pub fn callback(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::one_or_two);
  let callback = &definition.identifiers[0];
  let span = definition.identifiers.get(1);

  let invocation = if let Some(span) = span {
    quote! {
      let action = (parser.callbacks.#callback)(parser, parser.spans.#span.as_ptr(), parser.spans.#span.len());
    }
  } else {
    quote! {
      let action = (parser.callbacks.#callback)(parser, ptr::null(), 0);
    }
  };

  TokenStream::from(quote! {
    if parser.values.skip_next_callback == 0 {
      #invocation

      if action < 0 {
        return action;
      } else if action != 0 {
        return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
      }
    } else {
      parser.values.skip_next_callback = 0;
    }
  })
}

#[proc_macro]
pub fn suspend(_input: TokenStream) -> TokenStream {
  TokenStream::from(quote! { SUSPEND })
}

#[proc_macro]
pub fn find_method(_input: TokenStream) -> TokenStream {
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
    let method = match &parser.spans.method[..] {
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
pub fn match_state_string(_input: TokenStream) -> TokenStream {
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
pub fn match_error_code_string(_input: TokenStream) -> TokenStream {
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
pub fn values_getters(_input: TokenStream) -> TokenStream {
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
pub fn values_setters(_input: TokenStream) -> TokenStream {
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
pub fn spans_getters(_input: TokenStream) -> TokenStream {
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
pub fn callbacks_setters(_input: TokenStream) -> TokenStream {
  let spans_ref = SPANS.lock().unwrap();

  let mut callbacks: Vec<_> = CALLBACKS
    .lock()
    .unwrap()
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();

  for x in spans_ref.iter() {
    callbacks.push(format_ident!("on_data_{}", x));
  }

  let callbacks_setters: Vec<_> = callbacks
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

  let mut callbacks: Vec<_> = CALLBACKS
    .lock()
    .unwrap()
    .iter()
    .map(|x| format_ident!("{}", x))
    .collect();

  for x in spans.iter() {
    callbacks.push(format_ident!("on_data_{}", x));
  }

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
    #[inline(always)]
    fn noop(_parser: &mut Parser, _data: *const c_uchar, _len: usize) -> isize {
      0
    }

    pub const RESERVED_NEGATIVE_ADVANCES: isize = #RESERVED_NEGATIVE_ADVANCES;
    pub const SUSPEND: isize = #SUSPEND;
    pub const PAUSE: isize = #PAUSE;

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

      fn clear(&mut self) {
        #( self.#spans.clear() );*
      }
    }

    impl Callbacks {
      fn new() -> Callbacks {
        Callbacks {
          #( #callbacks: noop ),*
        }
      }

      fn clear(&mut self) {
        #( self.#callbacks = noop );*
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
