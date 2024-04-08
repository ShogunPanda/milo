use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::{Captures, Regex};
use syn::{parse_str, Arm, ItemConst};

use crate::definitions::{
  save_constants, CALLBACKS, CONNECTION_CLOSE, CONNECTION_KEEPALIVE, CONNECTION_UPGRADE, ERRORS,
  MESSAGE_TYPE_AUTODETECT, MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_RESPONSE, METHODS, STATES,
};

/// Generates all parser constants.
pub fn generate_constants() -> TokenStream {
  save_constants();

  let methods_consts: Vec<_> = METHODS
    .get()
    .unwrap()
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const METHOD_{}: u8 = {};", x.replace('-', "_"), i)).unwrap())
    .collect();

  let errors_consts: Vec<_> = ERRORS
    .get()
    .unwrap()
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const ERROR_{}: u8 = {};", x, i)).unwrap())
    .collect();

  let callbacks_consts: Vec<_> = CALLBACKS
    .get()
    .unwrap()
    .iter()
    .enumerate()
    .map(|(i, x)| {
      parse_str::<ItemConst>(&format!(
        "pub const CALLBACK_{}: u8 = {};",
        x.replace('-', "_").to_uppercase(),
        i
      ))
      .unwrap()
    })
    .collect();

  let states_ref = unsafe { STATES.get().unwrap() };

  let states_consts: Vec<_> = states_ref
    .iter()
    .enumerate()
    .map(|(i, x)| parse_str::<ItemConst>(&format!("pub const STATE_{}: u8 = {};", x.0, i)).unwrap())
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
    type StateHandler = fn (parser: &mut Parser, data: &[c_uchar], available: usize);

    #[no_mangle]
    pub type Callback = fn (&mut Parser, usize, usize);

    pub const DEBUG: bool = cfg!(debug_assertions);

    pub const MESSAGE_TYPE_AUTODETECT: u8 = #MESSAGE_TYPE_AUTODETECT;
    pub const MESSAGE_TYPE_REQUEST: u8 = #MESSAGE_TYPE_REQUEST;
    pub const MESSAGE_TYPE_RESPONSE: u8 = #MESSAGE_TYPE_RESPONSE;

    pub const CONNECTION_KEEPALIVE: u8 = #CONNECTION_KEEPALIVE;
    pub const CONNECTION_CLOSE: u8 = #CONNECTION_CLOSE;
    pub const CONNECTION_UPGRADE: u8 = #CONNECTION_UPGRADE;

    #(#methods_consts)*

    #(#errors_consts)*

    #(#callbacks_consts)*

    #(#states_consts)*

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
  })
}

/// Generates all parser enums.
pub fn generate_enums() -> TokenStream {
  let snake_matcher = Regex::new(r"_([a-z])").unwrap();

  let methods_ref = METHODS.get().unwrap();
  let errors_ref = ERRORS.get().unwrap();
  let callbacks_ref = CALLBACKS.get().unwrap();
  let states_ref = unsafe { STATES.get().unwrap() };

  let methods: Vec<_> = methods_ref
    .iter()
    .map(|x| format_ident!("{}", x.replace('-', "_")))
    .collect();

  let errors: Vec<_> = errors_ref.iter().map(|x| format_ident!("{}", x)).collect();

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

  let states: Vec<_> = states_ref.iter().map(|x| format_ident!("{}", x.0)).collect();

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

  let states_from: Vec<_> = states
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

  let states_into: Vec<_> = states
    .iter()
    .map(|x| parse_str::<Arm>(&format!("States::{} => \"{}\"", x, x)).unwrap())
    .collect();

  TokenStream::from(quote! {
    // MessageType and Connection reflects the constants in generate_constants
    // to allow easier interoperability, especially in WASM.
    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum MessageTypes {
      AUTODETECT,
      REQUEST,
      RESPONSE,
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum Connections {
      KEEPALIVE,
      CLOSE,
      UPGRADE,
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum Methods {
      #(#methods),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum Errors {
      #(#errors),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum Callbacks {
      #(#callbacks),*
    }

    #[repr(u8)]
    #[derive(Copy, Clone, Debug)]
    pub enum States {
      #(#states),*
    }

    impl TryFrom<u8> for MessageTypes {
      type Error = ();

      fn try_from(value: u8) -> Result<Self, ()> {
        match value {
          0 => Ok(MessageTypes::AUTODETECT),
          1 => Ok(MessageTypes::REQUEST),
          2 => Ok(MessageTypes::RESPONSE),
          _ => Err(())
        }
      }
    }

    impl TryFrom<u8> for Connections {
      type Error = ();

      fn try_from(value: u8) -> Result<Self, ()> {
        match value {
          0 => Ok(Connections::KEEPALIVE),
          1 => Ok(Connections::CLOSE),
          2 => Ok(Connections::UPGRADE),
          _ => Err(())
        }
      }
    }

    impl TryFrom<u8> for Methods {
      type Error = ();

      fn try_from(value: u8) -> Result<Self, ()> {
        match value {
          #(#methods_from),*,
          _ => Err(())
        }
      }
    }

    impl TryFrom<u8> for Errors {
      type Error = ();

      fn try_from(value: u8) -> Result<Self, ()> {
        match value {
          #(#errors_from),*,
          _ => Err(())
        }
      }
    }

    impl TryFrom<u8> for Callbacks {
      type Error = ();

      fn try_from(value: u8) -> Result<Self, ()> {
        match value {
          #(#callbacks_from),*,
          _ => Err(())
        }
      }
    }

    impl TryFrom<u8> for States {
      type Error = ();

      fn try_from(value: u8) -> Result<Self, ()> {
        match value {
          #(#states_from),*,
          _ => Err(())
        }
      }
    }

    impl From<MessageTypes> for &str {
      fn from(value: MessageTypes) -> Self {
        match value {
          MessageTypes::AUTODETECT => "AUTODETECT",
          MessageTypes::REQUEST => "REQUEST",
          MessageTypes::RESPONSE => "RESPONSE"
        }
      }
    }

    impl From<Connections> for &str {
      fn from(value: Connections) -> Self {
        match value {
          Connections::KEEPALIVE => "KEEPALIVE",
          Connections::CLOSE => "CLOSE",
          Connections::UPGRADE => "UPGRADE"
        }
      }
    }

    impl From<Methods> for &str {
      fn from(value: Methods) -> Self {
        match value {
          #(#methods_into),*
        }
      }
    }

    impl From<Errors> for &str {
      fn from(value: Errors) -> Self {
        match value {
          #(#errors_into),*
        }
      }
    }

    impl From<Callbacks> for &str {
      fn from(value: Callbacks) -> Self {
        match value {
          #(#callbacks_into),*
        }
      }
    }

    impl From<States> for &str {
      fn from(value: States) -> Self {
        match value {
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

    impl States {
      pub fn as_str(self) -> &'static str {
        self.into()
      }
    }
  })
}
