use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::{Captures, Regex};
use syn::{parse_str, Arm, ItemConst};

use crate::definitions::{
  save_constants, CALLBACKS, ERRORS, MAX_OFFSETS_COUNT, METHODS, OFFSETS, STATES, VALUES_OFFSETS, VALUES_SIZE,
};

/// Generates all parser constants.
pub fn generate_constants() -> TokenStream {
  save_constants();

  let methods_consts: Vec<_> = METHODS
    .get()
    .unwrap()
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const METHOD_{}: usize = {};", x.replace('-', "_"), i)).unwrap())
    .collect();

  let errors_consts: Vec<_> = unsafe {
    ERRORS
      .get()
      .unwrap()
      .iter()
      .enumerate()
      .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const ERROR_{}: usize = {};", x, i)).unwrap())
      .collect()
  };

  let callbacks_consts: Vec<_> = unsafe {
    CALLBACKS
      .get()
      .unwrap()
      .iter()
      .enumerate()
      .map(|(i, x)| {
        parse_str::<ItemConst>(&format!(
          "pub const CALLBACK_{}: usize = {};",
          x.replace('-', "_").to_uppercase(),
          i
        ))
        .unwrap()
      })
      .collect()
  };

  let offsets_consts: Vec<_> = unsafe {
    OFFSETS
      .get()
      .unwrap()
      .iter()
      .enumerate()
      .map(|(i, x)| {
        parse_str::<ItemConst>(&format!("pub const OFFSET_{}: usize = {};", x.replace('-', "_"), i)).unwrap()
      })
      .collect()
  };

  let values_ref = unsafe { VALUES_OFFSETS.get().unwrap() };
  let values_size = unsafe { VALUES_SIZE.get().unwrap() };
  let max_offsets_count = MAX_OFFSETS_COUNT;

  let states_ref = unsafe { STATES.get().unwrap() };

  let values_consts: Vec<_> = values_ref
    .iter()
    .map(|(k, v)| parse_str::<ItemConst>(&format!("pub const VALUE_{}: usize = {};", k.to_uppercase(), v)).unwrap())
    .collect();

  let states_consts: Vec<_> = states_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const STATE_{}: usize = {};", x, i)).unwrap())
    .collect();

  let states_len = states_ref.len();

  let states_table: Vec<_> = unsafe { STATES.get().unwrap() }
    .iter()
    .map(|x| format_ident!("state_{}", x.to_lowercase()))
    .collect();

  let digit_table: Vec<_> = (0..=255).map(|i| (0x30..=0x39).contains(&i)).collect();

  let hex_digit_table: Vec<_> = (0..=255)
    .map(|i| (0x30..=0x39).contains(&i) || (0x41..=0x46).contains(&i) || (0x61..=0x66).contains(&i))
    .collect();

  let token_other_characters = [
    b'!', b'#', b'$', b'%', b'&', b'\'', b'*', b'+', b'-', b'.', b'^', b'_', b'`', b',', b'~',
  ];

  let token_table: Vec<_> = (0..=255)
    .map(|i| {
      (0x30..=0x39).contains(&i)
        || (0x41..=0x5A).contains(&i)
        || (0x61..=0x7A).contains(&i)
        || token_other_characters.contains(&i)
    })
    .collect();

  let mut token_value_table: Vec<_> = (0..=255).map(|_| false).collect();
  token_value_table[9] = true;
  token_value_table[32] = true;

  for i in 0x21..=0xff {
    if i != 0x7f {
      token_value_table[i] = true;
    }
  }

  let mut token_value_quoted_table: Vec<_> = (0..=255).map(|_| false).collect();
  token_value_quoted_table[9] = true;
  token_value_quoted_table[32] = true;

  for i in 0x21..=0x7e {
    token_value_quoted_table[i] = true;
  }

  let url_other_characters = [
    b'-', b'.', b'_', b'~', b':', b'/', b'?', b'#', b'[', b']', b'@', b'!', b'$', b'&', b'\'', b'(', b')', b'*', b'+',
    b',', b';', b'=', b'%',
  ];
  let url_table: Vec<_> = (0..=255)
    .map(|i| {
      (0x30..=0x39).contains(&i)
        || (0x41..=0x5A).contains(&i)
        || (0x61..=0x7A).contains(&i)
        || url_other_characters.contains(&i)
    })
    .collect();

  let mut ws_table: Vec<_> = (0..=255).map(|_| false).collect();
  ws_table[9] = true;
  ws_table[32] = true;

  TokenStream::from(quote! {
    type StateHandler = fn (parser: &Parser, data: &[c_uchar]) -> isize;

    #[no_mangle]
    pub type Callback = fn (&Parser, usize, usize) -> isize;

    // We support 2048 offsets in total. This is usually fine most of the times.
    pub const MAX_OFFSETS_COUNT: usize = #max_offsets_count;
    pub const VALUES_SIZE: usize = #values_size;
    pub const SUSPEND: isize = isize::MIN;

    pub const DEBUG: bool = cfg!(debug_assertions);

    pub const AUTODETECT: usize = 0;
    pub const REQUEST: usize = 1;
    pub const RESPONSE: usize = 2;

    pub const CONNECTION_KEEPALIVE: usize = 0;
    pub const CONNECTION_CLOSE: usize = 1;
    pub const CONNECTION_UPGRADE: usize = 2;

    #(#methods_consts)*

    #(#errors_consts)*

    #(#callbacks_consts)*

    #(#states_consts)*

    #(#values_consts)*

    #(#offsets_consts)*

    /// cbindgen:ignore
    static DIGIT_TABLE: [bool; 256] = [#(#digit_table),*];

    /// cbindgen:ignore
    static HEX_DIGIT_TABLE: [bool; 256] = [#(#hex_digit_table),*];

    /// cbindgen:ignore
    static TOKEN_TABLE: [bool; 256] = [#(#token_table),*];

    /// cbindgen:ignore
    static TOKEN_VALUE_TABLE: [bool; 256] = [#(#token_value_table),*];

    /// cbindgen:ignore
    static TOKEN_VALUE_QUOTED_TABLE: [bool; 256] = [#(#token_value_quoted_table),*];

    /// cbindgen:ignore
    static URL_TABLE: [bool; 256] = [#(#url_table),*];

    /// cbindgen:ignore
    static WS_TABLE: [bool; 256] = [#(#ws_table),*];

    /// cbindgen:ignore
    static STATES_HANDLERS: [StateHandler; #states_len] = [#(#states_table),*];
  })
}

/// Generates all parser enums.
pub fn generate_enums() -> TokenStream {
  let snake_matcher = Regex::new(r"_([a-z])").unwrap();

  let methods_ref = METHODS.get().unwrap();
  let errors_ref = unsafe { ERRORS.get().unwrap() };
  let callbacks_ref = unsafe { CALLBACKS.get().unwrap() };
  let offsets_ref = unsafe { OFFSETS.get().unwrap() };
  let states_ref = unsafe { STATES.get().unwrap() };

  let methods: Vec<_> = methods_ref
    .iter()
    .map(|x| format_ident!("{}", x.replace('-', "_")))
    .collect();

  let errors: Vec<_> = errors_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let offsets: Vec<_> = offsets_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let callbacks: Vec<_> = callbacks_ref
    .iter()
    .map(|x| {
      let lowercase = x.to_lowercase();

      format_ident!(
        "{}",
        snake_matcher.replace_all(lowercase.as_str(), |captures: &Captures| captures[1].to_uppercase())
      )
    })
    .collect();

  let states: Vec<_> = states_ref.iter().map(|x| format_ident!("{}", x)).collect();

  let methods_from: Vec<_> = methods_ref
    .iter()
    .enumerate()
    .map(|(x, i)| parse_str::<Arm>(&format!("{} => Ok(Methods::{})", x, i.replace('-', "_"))).unwrap())
    .collect();

  let errors_from: Vec<_> = errors_ref
    .iter()
    .enumerate()
    .map(|(x, i)| parse_str::<Arm>(&format!("{} => Ok(Errors::{})", x, i)).unwrap())
    .collect();

  let callbacks_from: Vec<_> = callbacks
    .iter()
    .enumerate()
    .map(|(x, i)| parse_str::<Arm>(&format!("{} => Ok(Callbacks::{})", x, i)).unwrap())
    .collect();

  let offsets_from: Vec<_> = offsets_ref
    .iter()
    .enumerate()
    .map(|(x, i)| parse_str::<Arm>(&format!("{} => Ok(Offsets::{})", x, i.replace('-', "_"))).unwrap())
    .collect();

  let states_from: Vec<_> = states_ref
    .iter()
    .enumerate()
    .map(|(x, i)| parse_str::<Arm>(&format!("{} => Ok(States::{})", x, i)).unwrap())
    .collect();

  let methods_into: Vec<_> = methods_ref
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Methods::{} => \"{}\"", x.replace('-', "_"), x)).unwrap())
    .collect();

  let errors_into: Vec<_> = errors_ref
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Errors::{} => \"{}\"", x, x)).unwrap())
    .collect();

  let callbacks_into: Vec<_> = callbacks
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Callbacks::{} => \"{}\"", x, x)).unwrap())
    .collect();

  let offsets_into: Vec<_> = offsets_ref
    .iter()
    .map(|x| parse_str::<Arm>(&format!("Offsets::{} => \"{}\"", x.replace('-', "_"), x)).unwrap())
    .collect();

  let states_into: Vec<_> = states_ref
    .iter()
    .map(|x| parse_str::<Arm>(&format!("States::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    // MessageType and Connection reflects the constants in generate_constants
    // to allow easier interoperability, especially in WASM.
    #[wasm_bindgen]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug)]
    pub enum MessageTypes {
      AUTODETECT,
      REQUEST,
      RESPONSE,
    }

    #[wasm_bindgen]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug)]
    pub enum Connections {
      KEEPALIVE,
      CLOSE,
      UPGRADE,
    }

    #[wasm_bindgen]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug)]
    pub enum Methods {
      #(#methods),*
    }

    #[wasm_bindgen]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug)]
    pub enum Errors {
      #(#errors),*
    }

    #[wasm_bindgen]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug)]
    pub enum Callbacks {
      #(#callbacks),*
    }

    #[wasm_bindgen]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug)]
    pub enum Offsets {
      #(#offsets),*
    }

    #[wasm_bindgen]
    #[repr(usize)]
    #[derive(Copy, Clone, Debug)]
    pub enum States {
      #(#states),*
    }

    impl TryFrom<usize> for MessageTypes {
      type Error = ();

      fn try_from(value: usize) -> Result<Self, ()> {
        match value {
          0 => Ok(MessageTypes::AUTODETECT),
          1 => Ok(MessageTypes::REQUEST),
          2 => Ok(MessageTypes::RESPONSE),
          _ => Err(())
        }
      }
    }

    impl TryFrom<usize> for Connections {
      type Error = ();

      fn try_from(value: usize) -> Result<Self, ()> {
        match value {
          0 => Ok(Connections::KEEPALIVE),
          1 => Ok(Connections::CLOSE),
          2 => Ok(Connections::UPGRADE),
          _ => Err(())
        }
      }
    }

    impl TryFrom<usize> for Methods {
      type Error = ();

      fn try_from(value: usize) -> Result<Self, ()> {
        match value {
          #(#methods_from),*,
          _ => Err(())
        }
      }
    }

    impl TryFrom<usize> for Errors {
      type Error = ();

      fn try_from(value: usize) -> Result<Self, ()> {
        match value {
          #(#errors_from),*,
          _ => Err(())
        }
      }
    }

    impl TryFrom<usize> for Callbacks {
      type Error = ();

      fn try_from(value: usize) -> Result<Self, ()> {
        match value {
          #(#callbacks_from),*,
          _ => Err(())
        }
      }
    }

    impl TryFrom<usize> for Offsets {
      type Error = ();

      fn try_from(value: usize) -> Result<Self, ()> {
        match value {
          #(#offsets_from),*,
          _ => Err(())
        }
      }
    }

    impl TryFrom<usize> for States {
      type Error = ();

      fn try_from(value: usize) -> Result<Self, ()> {
        match value {
          #(#states_from),*,
          _ => Err(())
        }
      }
    }

    impl Into<&str> for MessageTypes {
      fn into(self) -> &'static str {
        match self {
          MessageTypes::AUTODETECT => "AUTODETECT",
          MessageTypes::REQUEST => "REQUEST",
          MessageTypes::RESPONSE => "RESPONSE"
        }
      }
    }

    impl Into<&str> for Connections {
      fn into(self) -> &'static str {
        match self {
          Connections::KEEPALIVE => "KEEPALIVE",
          Connections::CLOSE => "CLOSE",
          Connections::UPGRADE => "UPGRADE"
        }
      }
    }

    impl Into<&str> for Methods {
      fn into(self) -> &'static str {
        match self {
          #(#methods_into),*
        }
      }
    }

    impl Into<&str> for Errors {
      fn into(self) -> &'static str {
        match self {
          #(#errors_into),*
        }
      }
    }

    impl Into<&str> for Offsets {
      fn into(self) -> &'static str {
        match self {
          #(#offsets_into),*,
        }
      }
    }

    impl Into<&str> for Callbacks {
      fn into(self) -> &'static str {
        match self {
          #(#callbacks_into),*
        }
      }
    }

    impl Into<&str> for States {
      fn into(self) -> &'static str {
        match self {
          #(#states_into),*
        }
      }
    }

    impl MessageTypes {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }

    impl Connections {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }

    impl Methods {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }

    impl Errors {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }

    impl Callbacks {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }

    impl Offsets {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }

    impl States {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }
  })
}
