use proc_macro::TokenStream;
use quote::quote;

mod actions;
mod definitions;
mod generators;
mod matchers;
mod native;
mod parsing;
mod wasm;

// #region definitions
#[proc_macro]
pub fn measure(input: TokenStream) -> TokenStream { definitions::measure(input) }

#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream { definitions::state(input) }
// #endregion definitions

// #region matchers
#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream { matchers::char(input) }

#[proc_macro]
pub fn digit(_: TokenStream) -> TokenStream { matchers::digit() }

#[proc_macro]
pub fn hex_digit(_: TokenStream) -> TokenStream { matchers::hex_digit() }

#[proc_macro]
pub fn string(input: TokenStream) -> TokenStream { matchers::string(input) }

#[proc_macro]
pub fn case_insensitive_string(input: TokenStream) -> TokenStream { matchers::case_insensitive_string(input) }

#[proc_macro]
pub fn crlf(_: TokenStream) -> TokenStream { matchers::crlf() }

#[proc_macro]
pub fn double_crlf(_: TokenStream) -> TokenStream { matchers::double_crlf() }

#[proc_macro]
pub fn token(_: TokenStream) -> TokenStream { matchers::token() }

#[proc_macro]
pub fn token_value(_: TokenStream) -> TokenStream { matchers::token_value() }

#[proc_macro]
pub fn method(input: TokenStream) -> TokenStream { matchers::method(input) }

#[proc_macro]
pub fn url(_: TokenStream) -> TokenStream { matchers::url() }

#[proc_macro]
pub fn otherwise(input: TokenStream) -> TokenStream { matchers::otherwise(input) }

#[proc_macro]
pub fn process_state(_: TokenStream) -> TokenStream { actions::process_state() }
// #endregion matchers

// #region values access
#[proc_macro]
pub fn wasm_getter(input: TokenStream) -> TokenStream { wasm::wasm_getter(input) }
// #endregion values access

// #region actions
#[proc_macro]
pub fn string_length(input: TokenStream) -> TokenStream { actions::string_length(input) }

#[proc_macro]
pub fn advance(input: TokenStream) -> TokenStream { actions::advance(input) }

#[proc_macro]
pub fn move_to(input: TokenStream) -> TokenStream { actions::move_to(input) }

#[proc_macro]
pub fn fail(input: TokenStream) -> TokenStream { actions::fail(input) }

#[proc_macro]
pub fn consume(input: TokenStream) -> TokenStream { actions::consume(input) }

#[proc_macro]
pub fn callback(input: TokenStream) -> TokenStream { actions::callback(input) }

#[proc_macro]
pub fn suspend(_: TokenStream) -> TokenStream { actions::suspend() }

#[proc_macro]
pub fn r#return(_: TokenStream) -> TokenStream { actions::r#return() }

#[proc_macro]
pub fn find_method(input: TokenStream) -> TokenStream { actions::find_method(input) }
// #endregion actions

// #region generators
#[proc_macro]
pub fn init_constants(_: TokenStream) -> TokenStream {
  definitions::init_constants();

  TokenStream::from(quote! {})
}

#[proc_macro]
pub fn generate_constants(_: TokenStream) -> TokenStream { generators::generate_constants() }

#[proc_macro]
pub fn generate_enums(_: TokenStream) -> TokenStream { generators::generate_enums() }

#[proc_macro]
pub fn generate_callbacks(_: TokenStream) -> TokenStream { native::generate_callbacks_native() }

#[proc_macro]
pub fn link_callbacks(_: TokenStream) -> TokenStream { wasm::link_callbacks() }
// #endregion generators
