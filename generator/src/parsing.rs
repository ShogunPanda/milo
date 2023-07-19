use syn::parse::{ Parse, ParseStream };
use syn::{ Block, Ident, LitByte, LitChar, LitInt, Result, Stmt, Token, LitStr };

pub enum Char {
  MATCH(LitByte),
  ASSIGNMENT(Ident),
}

pub struct CharRange {
  pub identifier: Ident,
  pub from: LitByte,
  pub to: LitByte,
}

pub struct Definition {
  pub identifiers: Vec<Ident>,
  pub next: Option<Ident>,
  pub advance: isize,
}

pub struct State {
  pub name: Ident,
  pub statements: Vec<Stmt>,
}

pub struct Failure {
  pub ident: Ident,
  pub message: LitStr,
}

impl Parse for Char {
  fn parse(input: ParseStream) -> Result<Self> {
    if input.peek(Ident) {
      let identifier: Ident = input.parse()?;

      return Ok(Char::ASSIGNMENT(identifier));
    }

    // Get the state name
    let c: LitChar = input.parse()?;

    Ok(Char::MATCH(LitByte::new(u8::try_from(c.value()).unwrap(), c.span())))
  }
}

impl Parse for CharRange {
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the state name
    let identifier: Ident = input.parse()?;
    input.parse::<Token![,]>()?;

    // Get the range
    let from: LitChar = input.parse()?;
    input.parse::<Token![,]>()?;
    let to: LitChar = input.parse()?;

    Ok(CharRange {
      identifier,
      from: LitByte::new(u8::try_from(from.value()).unwrap(), from.span()),
      to: LitByte::new(u8::try_from(to.value()).unwrap(), to.span()),
    })
  }
}

impl Definition {
  pub fn one(input: ParseStream) -> Result<Self> {
    Definition::parse(input, 1, 1)
  }

  pub fn one_or_two(input: ParseStream) -> Result<Self> {
    Definition::parse(input, 1, 2)
  }

  pub fn two(input: ParseStream) -> Result<Self> {
    Definition::parse(input, 2, 2)
  }

  pub fn identifiers_only(input: ParseStream) -> Result<Self> {
    let mut identifiers: Vec<Ident> = vec![];

    while !input.is_empty() {
      identifiers.push(input.parse()?);

      if !input.is_empty() {
        input.parse::<Token![,]>()?;
      }
    }

    Ok(Definition { identifiers, next: None, advance: 0 })
  }

  fn parse(input: ParseStream, min: usize, max: usize) -> Result<Self> {
    let mut identifiers: Vec<Ident> = vec![];
    let mut next: Option<Ident> = None;
    let mut advance: isize = 1; // By default we consume a character

    if input.is_empty() {
      return Err(syn::Error::new(input.span(), format!("expected {} identifiers", min)));
    }

    // Until we have input and havent encountered a @, let's collect identifiers
    while !input.is_empty() {
      // Check if there is a numeric literal at the end
      if input.peek(Token![@]) {
        if identifiers.len() < min {
          return Err(input.error(format!("expected at laest {} identifiers", min)));
        }

        input.parse::<Token![@]>()?;
        break;
      }

      // Append the next identifier
      identifiers.push(input.parse::<Ident>()?);

      // Skip the eventual comma
      if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
      }
    }

    if identifiers.len() < min {
      return Err(input.error(format!("expected at least {} identifiers", min)));
    } else if identifiers.len() > max {
      return Err(input.error(format!("expected at most {} identifiers", max)));
    }

    // Now get the eventual state
    if input.peek(Ident) {
      next = Some(input.parse()?);

      // Skip the eventual comma
      if input.peek(Token![,]) {
        input.parse::<Token![,]>()?;
      }
    }

    // Now get the advance
    if !input.is_empty() {
      // Get the value
      let lit: LitInt = input.parse()?;
      advance = lit.base10_parse::<isize>()?;
    }

    Ok(Definition {
      identifiers,
      next,
      advance,
    })
  }
}

impl Parse for State {
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the state name
    let name: Ident = input.parse()?;

    // Skip the comma
    input.parse::<Token![,]>()?;

    // Get the body
    let body: Block = input.parse()?;

    Ok(State {
      name,
      statements: body.stmts,
    })
  }
}

impl Parse for Failure {
  fn parse(input: ParseStream) -> Result<Self> {
    // Get the code
    let ident: Ident = input.parse()?;

    // Discard the comma
    input.parse::<Token![,]>()?;

    // Get the message
    let message: LitStr = input.parse()?;

    Ok(Failure { ident, message })
  }
}
