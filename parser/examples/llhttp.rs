use std::{fs::read_to_string, io::Write};

use comfy_table::{Attribute, Cell, Table, modifiers::UTF8_ROUND_CORNERS};

use crate::helpers::llhttp::{Body, Payload, TestCase, list_tests, load_test, parse_input, run_test};

#[path = "../tests/helpers/mod.rs"]
mod helpers;

const TABLE_BORDERS: &str = "││──├─┼┤│    ┬┴┬┴┌┐└┘";
const HIGHLIGHTED_CRLF: &str = "\x1b[1m\x1b[33m\\r\\n\n\x1b[0m";

fn check_test(path: String) {
  let raw = read_to_string(&path).unwrap();
  let mut test: TestCase = serde_yaml::from_str(&raw).unwrap();

  test.checked = true;
  serde_yaml::to_writer(std::fs::File::create(path.clone()).unwrap(), &test).unwrap();
}

fn show_test(case: TestCase) {
  let section = "requests";

  println!("Path: \x1b[1m{}\x1b[0m", case.path);
  println!(
    "LLHTTP Location: \x1b[1m{}:{}\x1b[0m",
    case.source.path, case.source.line
  );
  println!(
    "\nTest: \x1b[1m{}\x1b[0m ({})",
    case.name,
    if case.checked { "checked" } else { "unchecked" }
  );

  println!(
    "\n--- INPUT  ---\n{}",
    parse_input(&case.input).replace("\r\n", HIGHLIGHTED_CRLF) + HIGHLIGHTED_CRLF
  );

  println!("--- OUTPUT ---");

  // Build and show the table
  let mut table = Table::new();
  table
    .load_preset(TABLE_BORDERS)
    .apply_modifier(UTF8_ROUND_CORNERS)
    .set_header(vec![
      Cell::new("LLHTTP").add_attribute(Attribute::Bold),
      Cell::new("Milo").add_attribute(Attribute::Bold),
    ]);

  let llhttp: Vec<String> = case.llhttp.into_iter().filter(|l| !l.ends_with(" complete")).collect();

  // Serialize the Milo output to a llhttp-like format for comparison
  let milo = case
    .output
    .unwrap()
    .iter()
    .map(|event| {
      let offset = event.offset;

      if event.kind == "request" || event.kind == "response" {
        return None;
      }

      match &event.payload {
        Some(payload) => {
          match payload {
            Payload::String(payload) => {
              Some(format!(
                "off={} len={} span[{}]=\"{}\"",
                offset,
                payload.len(),
                event.kind,
                payload
              ))
            }
            Payload::Error(error) => {
              Some(format!(
                "off={} error_code={} error_description=\"{}\"",
                offset, error.code, error.description
              ))
            }
            Payload::Headers(headers) => {
              let body = if let Some(body) = &headers.body {
                match body {
                  Body::String(b) => b.into(),
                  Body::Number(n) => n.to_string(),
                }
              } else {
                String::from("none")
              };

              if section == "requests" {
                let method = headers.method.as_ref().unwrap();
                let url = headers.url.as_ref().unwrap();

                Some(format!(
                  "off={} headers method=\"{}\" url=\"{}\" protocol=\"{}\" version=\"{}\" body=\"{}\"",
                  offset, method, url, headers.protocol, headers.version, body
                ))
              } else {
                Some(format!(
                  "off={} headers status=\"{}\" protocol=\"{}\" version=\"{}\" body=\"{}\"",
                  offset,
                  headers.status.unwrap(),
                  headers.protocol,
                  headers.version,
                  body
                ))
              }
            }
          }
        }
        _ => {
          return Some(format!("off={} {}", event.offset, event.kind));
        }
      }
    })
    .filter(Option::is_some)
    .map(Option::unwrap)
    .collect::<Vec<String>>();

  let lines = llhttp.len().max(milo.len());
  let width = llhttp
    .iter()
    .chain(milo.iter())
    .map(|line| line.len())
    .max()
    .unwrap_or(0);

  for i in 0..lines {
    let llhttp_line = llhttp.get(i).map(String::as_str).unwrap_or("");
    let milo_line = milo.get(i).map(String::as_str).unwrap_or("");

    table.add_row(vec![
      Cell::new(format!("{:<width$}", llhttp_line, width = width)),
      Cell::new(format!("{:<width$}", milo_line, width = width)),
    ]);
  }

  println!("\n{}\n", table);
}

fn review_tests(section: &str) {
  let tests = list_tests(section);

  println!("{:#?}", tests);

  let unchecked_test: Vec<TestCase> = tests.into_iter().filter(|t| !t.checked).collect();

  let mut index = 0;
  let total = unchecked_test.len();
  for test in unchecked_test {
    index += 1;
    print!("\x1bc");
    println!("--- TEST {} of {} ---", index, total);
    let path = test.path.clone();
    show_test(test);

    print!("Do you want to mark this test as checked? (y/yes/n/no/q/quit) ");
    std::io::stdout().flush().unwrap();
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    match input.trim().to_lowercase().as_str() {
      "y" | "yes" => {
        check_test(path);
      }
      "q" | "quit" => {
        println!("Exiting review...");
        break;
      }
      _ => {}
    }
  }
}

fn main() {
  let args = std::env::args().collect::<Vec<_>>();

  if args.len() < 2 {
    println!("Usage:");
    println!("  --generate <section> <test>   Generate the output for a test");
    println!("  --review <section>            Review all tests in a section");
    println!("  --show <section> <test>       Show the input and output of a test");

    return;
  }

  match args.get(1).unwrap().as_str() {
    "--generate" => {
      let result = run_test(args.get(2).unwrap(), args.get(3).unwrap());

      print!("---\n{}", result.actual);
    }
    "--review" => {
      review_tests(args.get(2).unwrap());
    }
    "--show" => {
      show_test(load_test(args.get(2).unwrap(), args.get(3).unwrap()));
    }
    arg => {
      panic!("Unknown argument: {}", arg);
    }
  }
}
