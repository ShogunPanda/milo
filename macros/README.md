# milo-macros

milo-macros is an internal crate which represents the core of Milo.

It leverages Rust's [procedural macro], [syn] and [quote] crates to allow an easy definition of states and matchers for the parser.

To define a new macro, you have to define a new method which the following signature:

```rust
#[proc_macro]
pub fn new_macro(input: TokenStream) -> TokenStream {
  TokenStream::new( /* ... */)
}
```

It is important to notice that everything which is is not included **within** the return value will not be present in the execute code. This means that macros performance are not really important as they will only be evaluated at compile time.

To return a new code, leverage the `quote!` macro, while for parsing use the `parse_macro_input!` macro. Here's an example:

```rust
#[proc_macro]
pub fn char(input: TokenStream) -> TokenStream {
  let character = parse_macro_input!(input as LitChar);
  let byte = LitByte::new(c_uchar::try_from(character.value()).unwrap(), character.span());

  TokenStream::from(quote! { #byte })
}
```

[procedural macro]: https://doc.rust-lang.org/reference/procedural-macros.html
[syn]: https://crates.io/crates/syn
[quote]: https://crates.io/crates/quote
