use syn::parse::{Parse, ParseStream};
use syn::{Block, Expr, Ident, LitInt, LitStr, Result, Stmt, Token};

pub struct Identifiers {
  pub identifiers: Vec<Ident>,
}

pub struct IdentifiersWithExpr {
  pub identifier: Ident,
  pub expr: Option<Expr>,
}

pub struct Failure {
  pub error: Ident,
  pub message: Expr,
}

pub struct State {
  pub name: Ident,
  pub statements: Vec<Stmt>,
}

pub struct StringLength {
  pub string: LitStr,
  pub modifier: isize,
}

impl Identifiers {
  pub fn unbound(input: ParseStream) -> Result<Self> {
    Identifiers::parse(input, 0, usize::MAX)
  }

  fn parse(input: ParseStream, min: usize, max: usize) -> Result<Self> {
    let mut identifiers = vec![];

    while !input.is_empty() {
      identifiers.push(input.parse()?);

      if !input.is_empty() {
        input.parse::<Token![,]>()?;
      }
    }

    if identifiers.len() < min {
      return Err(input.error(format!("expected at least {} identifiers", min)));
    } else if identifiers.len() > max {
      return Err(input.error(format!("expected at most {} identifiers", max)));
    }

    Ok(Identifiers { identifiers })
  }
}

impl Parse for Failure {
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
  fn parse(input: ParseStream) -> Result<Self> {
    let identifier = input.parse()?;
    let mut expr = None;

    if !input.is_empty() {
      // Discard the comma
      input.parse::<Token![,]>()?;

      expr = Some(input.parse::<Expr>()?);
    }

    Ok(IdentifiersWithExpr { identifier, expr })
  }
}

impl Parse for State {
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
  fn parse(input: ParseStream) -> Result<Self> {
    let string = input.parse()?;
    let mut modifier = 0;

    if !input.is_empty() {
      // Discard the comma
      input.parse::<Token![,]>()?;

      modifier = input.parse::<LitInt>()?.base10_parse::<isize>()?;
    }

    Ok(StringLength { string, modifier })
  }
}
