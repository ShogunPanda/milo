use std::fs::{read_to_string, File, OpenOptions};
use std::io::BufWriter;
use std::path::Path;
use std::sync::OnceLock;

use indexmap::IndexMap;
use proc_macro::TokenStream;
use quote::quote;
use semver::Version;
use serde::Serialize;
use syn::parse_macro_input;
use toml::Table;

use crate::parsing::IdentifierWithStatements;

// Global state variables for later use
pub static METHODS: OnceLock<Vec<String>> = OnceLock::new();
pub static ERRORS: OnceLock<Vec<String>> = OnceLock::new();
pub static CALLBACKS: OnceLock<Vec<String>> = OnceLock::new();
pub static mut STATES: OnceLock<Vec<(String, String)>> = OnceLock::new();

// Global constants
pub const MESSAGE_TYPE_AUTODETECT: u8 = 0;
pub const MESSAGE_TYPE_REQUEST: u8 = 1;
pub const MESSAGE_TYPE_RESPONSE: u8 = 2;

pub const CONNECTION_KEEPALIVE: u8 = 0;
pub const CONNECTION_CLOSE: u8 = 1;
pub const CONNECTION_UPGRADE: u8 = 2;

#[derive(Serialize)]
struct BuildInfo {
  version: IndexMap<String, u8>,
  constants: IndexMap<String, u8>,
}

pub fn init_constants() {
  let mut absolute_path = Path::new(file!())
    .canonicalize()
    .unwrap()
    .parent()
    .unwrap()
    .to_path_buf();
  absolute_path.push("../../parser/constants");
  absolute_path = absolute_path.canonicalize().unwrap();

  unsafe {
    absolute_path.push("methods.yml");
    let f = File::open(absolute_path.to_str().unwrap()).unwrap();
    absolute_path.pop();
    let methods = serde_yaml::from_reader(f).unwrap();

    absolute_path.push("errors.yml");
    let f = File::open(absolute_path.to_str().unwrap()).unwrap();
    absolute_path.pop();
    let errors = serde_yaml::from_reader(f).unwrap();

    absolute_path.push("callbacks.yml");
    let f = File::open(absolute_path.to_str().unwrap()).unwrap();
    absolute_path.pop();
    let callbacks = serde_yaml::from_reader(f).unwrap();

    let _ = METHODS.set(methods);
    let _ = ERRORS.set(errors);
    let _ = CALLBACKS.set(callbacks);
    let _ = STATES.set(Vec::new());
  }
}

// Export all build info to a file for the scripts to re-use it
pub fn save_constants() {
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
  let mut consts: IndexMap<String, u8> = IndexMap::new();
  consts.insert("MESSAGE_TYPE_AUTODETECT".into(), MESSAGE_TYPE_AUTODETECT);
  consts.insert("MESSAGE_TYPE_REQUEST".into(), MESSAGE_TYPE_REQUEST);
  consts.insert("MESSAGE_TYPE_RESPONSE".into(), MESSAGE_TYPE_RESPONSE);
  consts.insert("CONNECTION_KEEPALIVE".into(), CONNECTION_KEEPALIVE);
  consts.insert("CONNECTION_CLOSE".into(), CONNECTION_CLOSE);
  consts.insert("CONNECTION_UPGRADE".into(), CONNECTION_UPGRADE);

  for (i, x) in METHODS.get().unwrap().iter().enumerate() {
    consts.insert(format!("METHOD_{}", x.replace('-', "_")), i as u8);
  }

  for (i, x) in CALLBACKS.get().unwrap().iter().enumerate() {
    consts.insert(format!("CALLBACK_{}", x.to_uppercase()), i as u8);
  }

  for (i, x) in ERRORS.get().unwrap().iter().enumerate() {
    consts.insert(format!("ERROR_{}", x), i as u8);
  }

  for (i, x) in unsafe { STATES.get().unwrap() }.iter().enumerate() {
    consts.insert(format!("STATE_{}", x.0), i as u8);
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

/// Adds time measurement to a code block.
pub fn measure(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifierWithStatements);
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
pub fn state(input: TokenStream) -> TokenStream {
  let definition = parse_macro_input!(input as IdentifierWithStatements);
  let name = definition.name.to_string();
  let statements = definition.statements;

  unsafe { STATES.get_mut().unwrap() }.push((name.to_string().to_uppercase(), quote! { #(#statements)* }.to_string()));

  TokenStream::new()
}
