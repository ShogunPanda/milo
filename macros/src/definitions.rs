use std::fs::{read_to_string, File, OpenOptions};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use indexmap::{IndexMap, IndexSet};
use proc_macro::TokenStream;
use quote::{format_ident, quote};
use semver::Version;
use serde::Serialize;
use syn::parse_macro_input;
use toml::Table;

use crate::parsing::IdentifiersWithStatements;

// Global state variables for later use
pub static METHODS: OnceLock<Vec<String>> = OnceLock::new();
pub static mut ERRORS: OnceLock<IndexSet<String>> = OnceLock::new();
pub static mut CALLBACKS: OnceLock<IndexSet<String>> = OnceLock::new();
pub static mut OFFSETS: OnceLock<IndexSet<String>> = OnceLock::new();
pub static mut STATES: OnceLock<IndexSet<String>> = OnceLock::new();

#[derive(Serialize)]
struct BuildInfo {
  version: IndexMap<String, usize>,
  constants: IndexMap<String, usize>,
}

pub fn init_constants() {
  let mut absolute_path = PathBuf::from(file!());
  absolute_path.push("../../../parser/constants");
  absolute_path = absolute_path.canonicalize().unwrap();

  unsafe {
    METHODS.get_or_init(|| {
      absolute_path.push("methods.yml");
      let f = File::open(absolute_path.to_str().unwrap()).unwrap();
      absolute_path.pop();

      serde_yaml::from_reader(f).unwrap()
    });

    ERRORS.get_or_init(|| {
      absolute_path.push("errors.yml");
      let f = File::open(absolute_path.to_str().unwrap()).unwrap();
      absolute_path.pop();

      serde_yaml::from_reader(f).unwrap()
    });

    OFFSETS.get_or_init(|| {
      absolute_path.push("offsets.yml");
      let f = File::open(absolute_path.to_str().unwrap()).unwrap();
      absolute_path.pop();

      serde_yaml::from_reader(f).unwrap()
    });

    CALLBACKS.get_or_init(|| {
      absolute_path.push("callbacks.yml");
      let f = File::open(absolute_path.to_str().unwrap()).unwrap();
      absolute_path.pop();

      serde_yaml::from_reader(f).unwrap()
    });

    STATES.get_or_init(|| IndexSet::new());
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
  let mut version: IndexMap<String, usize> = IndexMap::new();
  version.insert("major".into(), milo_version.major as usize);
  version.insert("minor".into(), milo_version.minor as usize);
  version.insert("patch".into(), milo_version.patch as usize);

  // Serialize constants
  let mut consts: IndexMap<String, usize> = IndexMap::new();
  consts.insert("AUTODETECT".into(), 0);
  consts.insert("REQUEST".into(), 1);
  consts.insert("RESPONSE".into(), 2);
  consts.insert("CONNECTION_KEEPALIVE".into(), 0);
  consts.insert("CONNECTION_CLOSE".into(), 1);
  consts.insert("CONNECTION_UPGRADE".into(), 2);

  for (i, x) in METHODS.get().unwrap().iter().enumerate() {
    consts.insert(format!("METHOD_{}", x.replace('-', "_")), i);
  }

  for (i, x) in unsafe { CALLBACKS.get().unwrap() }.iter().enumerate() {
    consts.insert(format!("CALLBACKS_{}", x.to_uppercase()), i);
  }

  for (i, x) in unsafe { ERRORS.get().unwrap() }.iter().enumerate() {
    consts.insert(format!("ERROR_{}", x), i);
  }

  for (i, x) in unsafe { OFFSETS.get().unwrap() }.iter().enumerate() {
    consts.insert(format!("OFFSET_{}", x.replace('-', "_")), i);
  }

  for (i, x) in unsafe { STATES.get().unwrap() }.iter().enumerate() {
    consts.insert(format!("STATE_{}", x), i);
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
  let definition = parse_macro_input!(input as IdentifiersWithStatements);
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
  let definition = parse_macro_input!(input as IdentifiersWithStatements);
  let name = definition.name;
  let function = format_ident!("state_{}", name);
  let statements = definition.statements;

  unsafe {
    STATES.get_mut().unwrap().insert(name.to_string().to_uppercase());
  }

  TokenStream::from(quote! {
    #[inline(always)]
    pub fn #function (parser: &Parser, data: &[c_uchar]) -> isize {
      let mut data = data;
      #(#statements)*
    }
  })
}
