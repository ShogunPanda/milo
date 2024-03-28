use syn::parse::{Parse, ParseStream};
use syn::{Block, Expr, Ident, LitInt, LitStr, Result, Stmt, Token, Type};

/// An identifier associated to a message, typically a string - An example of
/// this is used in `fail!`.
pub struct Failure {
  pub error: Ident,
  pub message: Expr,
}

/// An identifier associated to an expression - An example of this is used in
/// `move!`.
pub struct IdentifiersWithExpr {
  pub identifier: Ident,
  pub expr: Option<Expr>,
}

/// An identifier associated to a list of statements.
pub struct IdentifiersWithStatements {
  pub name: Ident,
  pub statements: Vec<Stmt>,
}

/// A property specification. It is used by `wasm_getter!`.
pub struct Property {
  pub property: Ident,
  pub getter: Ident,
  pub r#type: Type,
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

impl Parse for IdentifiersWithStatements {
  // Parses a state definition
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the state name
    let name = input.parse()?;

    // Skip the comma
    input.parse::<Token![,]>()?;

    // Get the body
    let body = input.parse::<Block>()?;

    Ok(IdentifiersWithStatements {
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
      modifier = input.parse::<LitInt>()?.base10_parse::<usize>()?;
    }

    Ok(StringLength { string, modifier })
  }
}

impl Parse for Property {
  // Parses a property definition
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the name
    let property = input.parse()?;

    // Discard the comma
    input.parse::<Token![,]>()?;

    // Get the getter
    let getter = input.parse()?;

    // Discard the comma
    input.parse::<Token![,]>()?;

    // Get the type
    let r#type = input.parse()?;

    Ok(Property {
      property,
      getter,
      r#type,
    })
  }
}
