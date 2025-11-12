use proc_macro::TokenStream;

mod actions;
mod generators;
mod matchers;
mod native;
mod structs;
mod wasm;

// #region matchers
#[proc_macro]
pub fn crlf_new(_: TokenStream) -> TokenStream { matchers::crlf_new() }

#[proc_macro]
pub fn token_new(_: TokenStream) -> TokenStream { matchers::token_new() }

#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream { matchers::char(input) }

#[proc_macro]
pub fn string_length(input: TokenStream) -> TokenStream { matchers::string_length(input) }

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
pub fn url(_: TokenStream) -> TokenStream { matchers::url() }

#[proc_macro]
pub fn consume(input: TokenStream) -> TokenStream { matchers::consume(input) }

#[proc_macro]
pub fn otherwise(input: TokenStream) -> TokenStream { matchers::otherwise(input) }
// #endregion matchers

// #region actions
#[proc_macro]
pub fn state(input: TokenStream) -> TokenStream { actions::state(input) }

#[proc_macro]
pub fn advance(input: TokenStream) -> TokenStream { actions::advance(input) }

#[proc_macro]
pub fn move_to(input: TokenStream) -> TokenStream { actions::move_to(input) }

#[proc_macro]
pub fn fail(input: TokenStream) -> TokenStream { actions::fail(input) }

#[proc_macro]
pub fn callback(input: TokenStream) -> TokenStream { actions::callback(input) }

#[proc_macro]
pub fn suspend(_: TokenStream) -> TokenStream { actions::suspend() }

#[proc_macro]
pub fn parse_next(_: TokenStream) -> TokenStream { actions::parse_next() }
// #endregion actions

// #region generators
#[proc_macro]
pub fn generate(_: TokenStream) -> TokenStream { generators::generate() }
// #endregion generators
