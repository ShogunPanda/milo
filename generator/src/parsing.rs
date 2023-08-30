use syn::parse::{Parse, ParseStream};
use syn::{Block, Ident, LitByte, LitChar, LitInt, LitStr, Result, Stmt, Token};

pub struct Char {
  pub identifier: Option<Ident>,
  pub byte: Option<LitByte>,
}

pub struct CharRange {
  pub identifier: Ident,
  pub from: LitByte,
  pub to: LitByte,
}

pub struct Identifiers {
  pub identifiers: Vec<Ident>,
}

pub struct Failure {
  pub error: Ident,
  pub message: LitStr,
}

pub struct Move {
  pub state: Ident,
  pub advance: isize,
}

pub struct State {
  pub name: Ident,
  pub statements: Vec<Stmt>,
}

impl Parse for Char {
  fn parse(input: ParseStream) -> Result<Self> {
    let mut identifier = None;
    let mut byte = None;

    if input.peek(Ident) {
      identifier = Some(input.parse()?);

      if input.is_empty() {
        return Ok(Char { identifier, byte });
      } else {
        input.parse::<Token![@]>()?;
      }
    }

    // Get the state name
    let character = input.parse::<LitChar>()?;
    byte = Some(LitByte::new(u8::try_from(character.value()).unwrap(), character.span()));
    return Ok(Char { identifier, byte });
  }
}

impl Parse for CharRange {
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the state name
    let identifier = input.parse()?;
    input.parse::<Token![,]>()?;

    // Get the range
    let from = input.parse::<LitChar>()?;
    input.parse::<Token![,]>()?;
    let to = input.parse::<LitChar>()?;

    Ok(CharRange {
      identifier,
      from: LitByte::new(u8::try_from(from.value()).unwrap(), from.span()),
      to: LitByte::new(u8::try_from(to.value()).unwrap(), to.span()),
    })
  }
}

impl Identifiers {
  pub fn one(input: ParseStream) -> Result<Self> {
    Identifiers::parse(input, 1, 1)
  }

  pub fn one_or_two(input: ParseStream) -> Result<Self> {
    Identifiers::parse(input, 1, 2)
  }

  pub fn two(input: ParseStream) -> Result<Self> {
    Identifiers::parse(input, 2, 2)
  }

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

impl Parse for Move {
  fn parse(input: ParseStream) -> Result<Self> {
    let state = input.parse()?;
    let mut advance = 1;

    if !input.is_empty() {
      // Discard the comma
      input.parse::<Token![,]>()?;

      advance = input.parse::<LitInt>()?.base10_parse::<isize>()?;
    }

    Ok(Move { state, advance })
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
