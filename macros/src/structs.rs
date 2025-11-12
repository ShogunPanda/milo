use syn::parse::{Parse, ParseStream};
use syn::{Expr, Ident, LitInt, LitStr, Result, Token};

/// An identifier associated to a message, typically a string - An example of
/// this is used in `fail!`.
pub struct Failure {
  pub error: Ident,
  pub message: Expr,
}

/// An identifier associated to an expression - An example of this is used in
/// `move!`.
pub struct IdentifierWithExpr {
  pub identifier: Ident,
  pub expr: Option<Expr>,
}

/// A string length associated with a numeric modifier. It is used by
/// `string_length!`.
pub struct StringLength {
  pub string: LitStr,
  pub modifier: usize,
}

impl Parse for Failure {
  // Parses a failure definition
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the code
    let error = input.parse()?;

    // Discard the comma
    input.parse::<Token![,]>()?;

    // Get the message
    let message = input.parse()?;

    Ok(Failure { error, message })
  }
}

impl Parse for IdentifierWithExpr {
  // Parses a identifier and its optional expression
  fn parse(input: ParseStream) -> Result<Self> {
    let identifier = input.parse()?;
    let mut expr = None;

    // If there is more input
    if !input.is_empty() {
      // Discard the comma
      input.parse::<Token![,]>()?;

      // Parse the expression
      expr = Some(input.parse::<Expr>()?);
    }

    Ok(IdentifierWithExpr { identifier, expr })
  }
}

impl Parse for StringLength {
  // Parses a string length
  fn parse(input: ParseStream) -> Result<Self> {
    let string = input.parse()?;
    let mut modifier = 0;

    // If there is more input
    if !input.is_empty() {
      // Discard the comma
      input.parse::<Token![,]>()?;

      // Parse the modifier
      modifier = input.parse::<LitInt>()?.base10_parse::<usize>()?;
    }

    Ok(StringLength { string, modifier })
  }
}
