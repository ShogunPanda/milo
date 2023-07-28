use syn::parse::{Parse, ParseStream};
use syn::{Block, Expr, Ident, LitInt, LitStr, Result, Stmt, Token};

/// A plain list of idenitifiers.
pub struct Identifiers {
  pub identifiers: Vec<Ident>,
}

/// An identifier associated to an expression - An example of this is used in
/// `move!`.
pub struct IdentifiersWithExpr {
  pub identifier: Ident,
  pub expr: Option<Expr>,
}

/// An identifier associated to a message, typically a string - An example of
/// this is used in `fail!`.
pub struct Failure {
  pub error: Ident,
  pub message: Expr,
}

/// A parser state. It is made of a name identifier and a list of statements.
pub struct State {
  pub name: Ident,
  pub statements: Vec<Stmt>,
}

/// A string length associated with a numeric modifier. It is used by
/// `string_length!`.
pub struct StringLength {
  pub string: LitStr,
  pub modifier: isize,
}

impl Identifiers {
  // Parses any list of identifiers separated by commas.
  pub fn unbound(input: ParseStream) -> Result<Self> { Identifiers::parse(input, 0, usize::MAX) }

  // Parses a size bound list of identifiers separated by commas.
  fn parse(input: ParseStream, min: usize, max: usize) -> Result<Self> {
    let mut identifiers = vec![];

    // While there is more input
    while !input.is_empty() {
      // Parse the next identifier
      identifiers.push(input.parse()?);

      // If still not empty, discard the next commad
      if !input.is_empty() {
        input.parse::<Token![,]>()?;
      }
    }

    // Check against size buond
    if identifiers.len() < min {
      return Err(input.error(format!("expected at least {} identifiers", min)));
    } else if identifiers.len() > max {
      return Err(input.error(format!("expected at most {} identifiers", max)));
    }

    Ok(Identifiers { identifiers })
  }
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

impl Parse for IdentifiersWithExpr {
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

    Ok(IdentifiersWithExpr { identifier, expr })
  }
}

impl Parse for State {
  // Parses a state definition
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the state name
    let name = input.parse()?;

    // Skip the comma
    input.parse::<Token![,]>()?;

    // Get the body
    let body = input.parse::<Block>()?;

    Ok(State {
      name,
      statements: body.stmts,
    })
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
      modifier = input.parse::<LitInt>()?.base10_parse::<isize>()?;
    }

    Ok(StringLength { string, modifier })
  }
}
