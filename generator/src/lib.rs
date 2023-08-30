#[macro_use]
extern crate lazy_static;

mod parsing;

use indexmap::IndexSet;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use std::path::Path;
use std::sync::Mutex;
use syn::{parse_macro_input, parse_str, Arm, Block, ExprMethodCall, Ident, ItemConst, LitByte, LitInt, LitStr};

use parsing::{Char, CharRange, Failure, Identifiers, Move, State};

const RESERVED_NEGATIVE_ADVANCES: isize = isize::MIN + 10;
const SUSPEND: isize = RESERVED_NEGATIVE_ADVANCES - 1;
const PAUSE: isize = RESERVED_NEGATIVE_ADVANCES - 2;

lazy_static! {
  static ref METHODS: Mutex<Vec<String>> = {
    let mut absolute_path = Path::new(file!()).parent().unwrap().to_path_buf();
    absolute_path.push("methods.yml");
    let f = std::fs::File::open(absolute_path.to_str().unwrap()).unwrap();
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
    fn #name (parser: &mut Parser, data: &[u8]) -> isize { #(#statements)* }
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
  let definition = parse_macro_input!(input with Identifiers::unbound);

  let second_last_index = definition.identifiers.len() - 2;
  let callbacks: &[Ident] = &definition.identifiers[0..second_last_index];
  let source = definition.identifiers.get(second_last_index).unwrap();
  let len = definition.identifiers.last().unwrap();

  TokenStream::from(quote! {
    {
      let callbacks: Vec<_> = [#(parser.callbacks.#callbacks),*]
        .iter()
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .collect();

      for cb in callbacks.iter() {
        let action = cb(
          parser,
          #source as *const _ as *const i8,
          #len,
        );

        if action < 0 {
          return action;
        } else if action != 0 {
          return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
        }
      }

      parser.position += #len;
    }
  })
}

#[proc_macro]
pub fn get_span(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input with Identifiers::one);
  let span = &definition.identifiers[0];

  TokenStream::from(quote! { parser.get_span(&parser.spans.#span) })
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
    quote! { cb(parser, unsafe { std::ffi::CString::from_vec_unchecked(parser.spans.#span.clone()).as_c_str().as_ptr() }, parser.spans.#span.len()) }
  } else {
    quote! { cb(parser, std::ptr::null(), 0) }
  };

  TokenStream::from(quote! {
    if parser.values.skip_next_callback == 0 {
      if let Some(cb) = parser.callbacks.#callback {
        let action = #invocation;

        if action < 0 {
          return action;
        } else if action != 0 {
          return parser.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
        }
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
// #endregion actions

#[proc_macro]
pub fn generate_parser(input: TokenStream) -> TokenStream {
  let body_block = parse_macro_input!(input as Block);
  let body = body_block.stmts;

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
  let initial_state = format_ident!("{}", states_ref[0]);
  let states: Vec<_> = states_ref.iter().map(|x| format_ident!("{}", x)).collect();

  // Generate all the branches
  let states_arms: Vec<_> = states_ref
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
          ".field(\"{}\", &unsafe {{ std::ffi::CString::from_vec_unchecked(self.{}.clone()) }})",
          x, x
        )
      })
      .collect::<Vec<String>>()
      .join("")
  ))
  .unwrap();

  let callbacks_debug = parse_str::<ExprMethodCall>(&format!(
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
    use std::ffi::{CString,c_void};

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
      pub error_description: String
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
      #( pub #spans: Vec<u8> ),*
    }

    type ActiveCallback = fn (&mut Parser, *const c_char, usize) -> isize;
    // Do not use ActiveCallback here to ensure C headers are properly generated
    type Callback = Option<fn (&mut Parser, *const c_char, usize) -> isize>;

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
          owner: None,
          paused: false,
          state: State::#initial_state,
          position: 0,
          values: Values::new(),
          spans: Spans::new(),
          callbacks: Callbacks::new(),
          error_code: Error::NONE,
          error_description: String::new(),
        }
      }

      pub fn reset(&mut self, keep_position: bool) {
        self.state = State::#initial_state;
        self.paused = false;

        if !keep_position {
          self.position = 0;
        }
        self.values.clear();
        self.spans.clear();
        self.error_code = Error::NONE;
        self.error_description = String::new();
      }

      pub fn parse(&mut self, data: *const c_char, mut limit: usize) -> usize {
        if self.paused {
          return 0;
        }

        let mut aggregate: Vec<u8>;
        let unconsumed_len = self.spans.unconsumed.len();
        let mut consumed = 0;
        let mut current = unsafe { &*(std::slice::from_raw_parts(data, limit) as *const _ as *const [u8]) };

        if unconsumed_len > 0 {
          limit += unconsumed_len;
          aggregate = self.spans.unconsumed.clone();
          aggregate.extend_from_slice(current);
          current = &aggregate[..];
        }

        #[cfg(debug_assertions)]
        if self.position == 0 {
          if let Some(cb) = self.callbacks.before_state_change {
            if cb(self, std::ptr::null(), 0) > 0 {
              self.fail_str(Error::CALLBACK_ERROR, "Callback returned an error.");
            }
          }
        }

        current = &current[..limit];

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

              break;
            },
            _ => {}
          }

          /*
            Negative return values mean to consume N bytes and then pause.
            Returning PAUSE from a callback instructs to pause without consuming any byte.
          */
          if result < 0 {
            self.paused = true;

            if result < RESERVED_NEGATIVE_ADVANCES {
              // If SUSPEND was returned, it means the parser is not to be paused but there is not enough data yet
              self.paused = result == PAUSE;

              // Do not re-execute the callback when pausing due a callback
              self.values.skip_next_callback = if result == PAUSE { 1 } else { 0 };

              break;
            }

            let advance = -result as usize;
            self.position += advance;
            consumed += advance;
            break;
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

        if consumed < limit {
          self.spans.unconsumed = current.to_vec();
        } else {
          self.spans.unconsumed.clear();
        }

        consumed
      }

      #(#body)*
    }
  };

  TokenStream::from(output)
}

#[proc_macro]
pub fn generate_parser_interface(input: TokenStream) -> TokenStream {
  let body_block = parse_macro_input!(input as Block);
  let body = body_block.stmts;

  let states_to_string_arms: Vec<_> = STATES
    .lock()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("State::{} => \"{}\"", x, x)).unwrap())
    .collect();

  let error_to_string_arms: Vec<_> = ERRORS
    .lock()
    .unwrap()
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Error::{} => \"{}\"", x, x)).unwrap())
    .collect();

  let method_as_int_arms: Vec<_> = METHODS
    .lock()
    .unwrap()
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<Arm>(&format!("\"{}\" => {}", x, i)).unwrap())
    .collect();

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

  let user_writable_values_ref = USER_WRITABLE_VALUES.lock().unwrap();
  let values_setters: Vec<_> = user_writable_values_ref
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

  let spans_ref = SPANS.lock().unwrap();
  let spans_getters: Vec<_> = spans_ref
    .iter()
    .map(|span| {
      let getter = format_ident!("get_{}_string", span);
      let key = format_ident!("{}", span);

      quote! {
        #[no_mangle]
        pub extern "C" fn #getter(parser: *mut Parser) -> *mut c_char {
          unsafe { CString::from_vec_unchecked((*parser).spans.#key.clone()).into_raw() }
        }
      }
    })
    .collect();

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

  let output = quote! {
    #(#body)*

    #[no_mangle]
    pub extern "C" fn get_state_string(parser: *mut Parser) -> *mut c_char {
      let string = match unsafe { (*parser).state } {
        State::FINISH => "FINISH",
        State::ERROR => "ERROR",
        #(#states_to_string_arms),*
      };

      std::ffi::CString::new(string).unwrap().into_raw()
    }


    #[no_mangle]
    pub extern "C" fn get_error_code_string(parser: *mut Parser) -> *mut c_char {
      let string = match unsafe { (*parser).error_code } {
        Error::NONE => "NONE",
        Error::UNEXPECTED_DATA => "UNEXPECTED_DATA",
        Error::UNEXPECTED_EOF => "UNEXPECTED_EOF",
        Error::CALLBACK_ERROR => "CALLBACK_ERROR",

        #(#error_to_string_arms),*
      };

      std::ffi::CString::new(string).unwrap().into_raw()
    }

    #[no_mangle]
    pub extern "C" fn method_as_int(data: *mut c_char) -> isize {
      unsafe {
        match std::ffi::CStr::from_ptr(data).to_str().unwrap() {
          #(#method_as_int_arms),*,
          _ => -1,
        }
      }
    }

    #(#values_getters)*

    #(#values_setters)*

    #(#spans_getters)*

    #(#callbacks_setters)*
  };

  TokenStream::from(output)
}
