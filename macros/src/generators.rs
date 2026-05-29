use std::fs::{File, OpenOptions, read_to_string};
use std::io::BufWriter;
use std::path::Path;

use indexmap::IndexMap;
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use regex::{Captures, Regex};
use semver::Version;
use serde::Serialize;
use serde_json::Value;
use syn::{Arm, ItemConst, parse_str};
use toml::Table;

use crate::{native, wasm};

#[derive(Serialize)]
struct BuildInfo {
  version: IndexMap<String, u8>,
  constants: IndexMap<String, Value>,
}

fn init_constants() -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
  // Load from YAML files
  let mut absolute_path = Path::new(file!())
    .canonicalize()
    .unwrap()
    .parent()
    .unwrap()
    .to_path_buf();
  absolute_path.push("../../parser/constants");
  absolute_path = absolute_path.canonicalize().unwrap();

  absolute_path.push("methods.yml");
  let f = File::open(absolute_path.to_str().unwrap()).unwrap();
  absolute_path.pop();
  let methods: Vec<String> = serde_yaml::from_reader(f).unwrap();

  absolute_path.push("errors.yml");
  let f = File::open(absolute_path.to_str().unwrap()).unwrap();
  absolute_path.pop();
  let errors: Vec<String> = serde_yaml::from_reader(f).unwrap();

  absolute_path.push("callbacks.yml");
  let f = File::open(absolute_path.to_str().unwrap()).unwrap();
  absolute_path.pop();
  let callbacks: Vec<String> = serde_yaml::from_reader(f).unwrap();

  absolute_path.push("states.yml");
  let f = File::open(absolute_path.to_str().unwrap()).unwrap();
  absolute_path.pop();
  let states: Vec<String> = serde_yaml::from_reader(f).unwrap();

  (methods, errors, callbacks, states)
}

fn generate_constants_internal(items: &Vec<String>, prefix: &str) -> Vec<ItemConst> {
  let mut consts: Vec<ItemConst> = Vec::new();
  let mut bytes: Vec<&[u8]> = vec![];

  for (i, x) in items.iter().enumerate() {
    let uppercased = x.to_uppercase();
    let name = uppercased.replace('-', "_");
    bytes.push(x.as_bytes());

    consts.push(parse_str::<ItemConst>(&format!("pub const {}_{}: u8 = {};", prefix, name, i)).unwrap());
  }

  consts
}

fn generate_bitmask(items: &Vec<String>, prefix: &str) -> Vec<ItemConst> {
  let mut consts: Vec<ItemConst> = Vec::new();
  let mut all = 0u64;

  consts.push(parse_str::<ItemConst>(&format!("pub const {}_NONE: u64 = 0;", prefix)).unwrap());

  for (i, x) in items.iter().enumerate() {
    let uppercased = x.to_uppercase();
    let name = uppercased.replace('-', "_");
    let bit = 1 << i;
    all |= bit;

    consts.push(parse_str::<ItemConst>(&format!("pub const {}_{}: u64 = {};", prefix, name, bit)).unwrap());
  }

  consts.push(parse_str::<ItemConst>(&format!("pub const {}_ALL: u64 = {};", prefix, all)).unwrap());

  consts
}

fn generate_table<F>(validator: F) -> Vec<bool>
where
  F: Fn(u16) -> bool,
{
  (0..=255).map(validator).collect()
}

/// Generates all parser constants.
fn generate_constants(
  methods: &Vec<String>,
  errors: &Vec<String>,
  callbacks: &Vec<String>,
  states: &Vec<String>,
) -> TokenStream {
  let methods_consts = generate_constants_internal(&methods, "METHOD");
  let states_consts = generate_constants_internal(&states, "STATE");
  let errors_consts = generate_constants_internal(&errors, "ERROR");
  let callbacks_consts = generate_constants_internal(&callbacks, "CALLBACK");
  let callbacks_bitmask = generate_bitmask(&callbacks, "CALLBACK_ACTIVE");
  let token_table = generate_table(|byte| {
    (0x30..=0x39).contains(&byte)
      || (0x41..=0x5a).contains(&byte)
      || (0x61..=0x7a).contains(&byte)
      || matches!(
        byte as u8,
        b'!' | b'#' | b'$' | b'%' | b'&' | b'\'' | b'*' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
      )
  });
  let url_table = generate_table(|byte| {
    (0x30..=0x39).contains(&byte)
      || (0x41..=0x5a).contains(&byte)
      || (0x61..=0x7a).contains(&byte)
      || matches!(
        byte as u8,
        b'-'
          | b'.'
          | b'_'
          | b'~'
          | b':'
          | b'/'
          | b'?'
          | b'['
          | b']'
          | b'@'
          | b'!'
          | b'$'
          | b'&'
          | b'\''
          | b'('
          | b')'
          | b'*'
          | b'+'
          | b','
          | b';'
          | b'='
          | b'%'
      )
  });
  let quoted_string_table = generate_table(|byte| {
    byte == 0x09
      || byte == 0x20
      || byte == 0x21
      || (0x23..=0x5b).contains(&byte)
      || (0x5d..=0x7e).contains(&byte)
      || byte >= 0x80
  });
  let quoted_pair_table =
    generate_table(|byte| byte == 0x09 || byte == 0x20 || (0x21..=0x7e).contains(&byte) || byte >= 0x80);

  TokenStream::from(quote! {
    pub type StateHandler = fn (parser: &mut Parser, data: &[c_uchar], available: usize);

    #[unsafe(no_mangle)]
    pub type Callback = fn (&mut Parser, usize, usize);

    pub const DEBUG: bool = cfg!(debug_assertions);

    #(#methods_consts)*
    #(#errors_consts)*
    #(#callbacks_consts)*
    #(#callbacks_bitmask)*
    #(#states_consts)*

    /// cbindgen:ignore
    static TOKEN_TABLE: [bool; 256] = [#(#token_table),*];

    /// cbindgen:ignore
    static URL_TABLE: [bool; 256] = [#(#url_table),*];

    /// cbindgen:ignore
    static QUOTED_STRING_TABLE: [bool; 256] = [#(#quoted_string_table),*];

    /// cbindgen:ignore
    static QUOTED_PAIR_TABLE: [bool; 256] = [#(#quoted_pair_table),*];
  })
}

/// Generates all parser enums.
fn generate_enums(
  methods: &Vec<String>,
  errors: &Vec<String>,
  callbacks: &Vec<String>,
  states: &Vec<String>,
) -> TokenStream {
  let snake_matcher = Regex::new(r"_([a-z])").unwrap();

  let methods_ref = methods;
  let errors_ref = errors;
  let callbacks_ref = callbacks;
  let states_ref = states;

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

  let states: Vec<_> = states_ref
    .iter()
    .map(|x| format_ident!("{}", x.to_uppercase()))
    .collect();

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

fn generate_callbacks(callbacks: &Vec<String>) -> TokenStream {
  let native = native::generate_callbacks(callbacks);
  let wasm = wasm::generate_callbacks(callbacks);

  TokenStream::from_iter([native, wasm])
}

// Export all build info to a file for the scripts to re-use it
fn save_constants(methods: &Vec<String>, errors: &Vec<String>, callbacks: &Vec<String>, states: &Vec<String>) {
  let mut milo_cargo_toml_path = Path::new(file!()).parent().unwrap().to_path_buf();
  milo_cargo_toml_path.push("../../parser/Cargo.toml");

  // Get milo version
  let milo_cargo_toml = read_to_string(milo_cargo_toml_path).unwrap().parse::<Table>().unwrap();
  let milo_version = Version::parse(
    milo_cargo_toml["package"].as_table().unwrap()["version"]
      .as_str()
      .unwrap(),
  )
  .unwrap();
  let mut version: IndexMap<String, u8> = IndexMap::new();
  version.insert("major".into(), milo_version.major as u8);
  version.insert("minor".into(), milo_version.minor as u8);
  version.insert("patch".into(), milo_version.patch as u8);

  // Serialize constants
  let mut consts: IndexMap<String, Value> = IndexMap::new();
  consts.insert(
    "DEBUG".into(),
    if cfg!(debug_assertions) {
      Value::Bool(true)
    } else {
      Value::Bool(false)
    },
  );
  for (i, x) in methods.iter().enumerate() {
    consts.insert(format!("METHOD_{}", x.replace('-', "_")), i.into());
  }

  for (i, x) in callbacks.iter().enumerate() {
    consts.insert(format!("CALLBACK_{}", x.to_uppercase()), i.into());
  }

  let mut all = 0u64;
  consts.insert("CALLBACK_ACTIVE_NONE".into(), 0.into());
  for (i, x) in callbacks.iter().enumerate() {
    let bit = 1 << i;
    consts.insert(format!("CALLBACK_ACTIVE_{}", x.to_uppercase()), bit.into());
    all |= bit;
  }
  consts.insert("CALLBACK_ACTIVE_ALL".into(), all.into());

  for (i, x) in errors.iter().enumerate() {
    consts.insert(format!("ERROR_{}", x), i.into());
  }

  for (i, x) in states.iter().enumerate() {
    consts.insert(format!("STATE_{}", x.to_uppercase()), i.into());
  }

  // Prepare the data to save
  let data = BuildInfo {
    version,
    constants: consts,
  };

  // Write information in the file
  let mut json_path = Path::new(file!()).parent().unwrap().to_path_buf();
  json_path.push("../../parser/target/buildinfo.json");

  let file = OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .open(json_path.as_path());

  let writer = BufWriter::new(file.unwrap());
  serde_json::to_writer_pretty(writer, &data).unwrap();
}

/// Generates the complete parser.
pub fn generate() -> TokenStream {
  let (methods, errors, callbacks, states) = init_constants();

  let constants_code = generate_constants(&methods, &errors, &callbacks, &states);
  let enums_code = generate_enums(&methods, &errors, &callbacks, &states);
  let callbacks_code = generate_callbacks(&callbacks);
  save_constants(&methods, &errors, &callbacks, &states);

  TokenStream::from_iter([constants_code, enums_code, callbacks_code])
}
