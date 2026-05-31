use std::{
  fs::read_to_string,
  io::{Read, Write},
  path::{Component, Path},
};

use comfy_table::{Attribute, Cell, Table, modifiers::UTF8_ROUND_CORNERS};

use crate::helpers::llhttp::{Body, Event, Payload, TestCase, list_tests, load_test, run_test};

#[path = "../tests/helpers/mod.rs"]
mod helpers;

const TABLE_BORDERS: &str = "││──├─┼┤│    ┬┴┬┴┌┐└┘";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const BOLD_RED: &str = "\x1b[1m\x1b[31m";

enum InquirerDecision {
  Check,
  Skip,
  Quit,
}

struct Inquirer {
  original_termios: libc::termios,
}

impl Inquirer {
  fn new() -> Inquirer {
    unsafe {
      let mut original_termios = std::mem::zeroed::<libc::termios>();

      if libc::tcgetattr(libc::STDIN_FILENO, &mut original_termios) != 0 {
        panic!("Failed to get terminal attributes");
      }

      let mut current = original_termios;
      current.c_lflag &= !(libc::ICANON | libc::ECHO);
      current.c_cc[libc::VMIN] = 1;
      current.c_cc[libc::VTIME] = 0;

      if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &current) != 0 {
        panic!("Failed to set terminal attributes");
      }

      Inquirer { original_termios }
    }
  }

  fn query(&self) -> InquirerDecision {
    let mut input = [0; 1];

    std::io::stdin().read_exact(&mut input).unwrap();
    println!();

    match input[0] {
      b'y' | b'Y' => InquirerDecision::Check,
      b'q' | b'Q' => InquirerDecision::Quit,
      _ => InquirerDecision::Skip,
    }
  }
}

impl Drop for Inquirer {
  fn drop(&mut self) {
    unsafe {
      libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &self.original_termios);
    }
  }
}

fn section_from_fixture_path(path: &str) -> Option<&str> {
  let mut components = Path::new(path).components();

  while let Some(component) = components.next() {
    if component == Component::Normal("llhttp".as_ref()) {
      return components.next().and_then(|component| {
        match component {
          Component::Normal(section) => section.to_str(),
          _ => None,
        }
      });
    }
  }

  None
}

fn check_test(path: String) {
  let raw = read_to_string(&path).unwrap();
  let mut test: TestCase = serde_yaml::from_str(&raw).unwrap();

  test.checked = true;
  serde_yaml::to_writer(std::fs::File::create(path.clone()).unwrap(), &test).unwrap();
}

fn update_test(path: &str) {
  let raw = read_to_string(path).unwrap();
  let mut test: TestCase = serde_yaml::from_str(&raw).unwrap();
  let section = section_from_fixture_path(path).unwrap_or("unknown");
  let result = run_test(section, path);

  test.output = Some(serde_yaml::from_str::<Vec<Event>>(&result.actual).unwrap());
  serde_yaml::to_writer(std::fs::File::create(path).unwrap(), &test).unwrap();
}

fn push_wrapped(output: &mut String, column: &mut usize, visible_len: usize, text: &str) {
  if *column > 0 && *column + visible_len > 80 {
    output.push('\n');
    *column = 0;
  }

  output.push_str(text);
  *column += visible_len;
}

fn highlight_input(input: &str) -> String {
  let bytes = input.as_bytes();
  let mut output = String::new();
  let mut column = 0usize;
  let mut i = 0;

  while i < bytes.len() {
    if bytes[i] != b'\\' {
      if bytes[i] == b'\n' {
        output.push('\n');
        column = 0;
      } else {
        push_wrapped(&mut output, &mut column, 1, &(bytes[i] as char).to_string());
      }

      i += 1;
      continue;
    }

    if bytes.get(i..i + 4) == Some(b"\\r\\n") {
      push_wrapped(&mut output, &mut column, 4, &format!("{}\\r\\n{}", BOLD, RESET));
      i += 4;
    } else if bytes.get(i..i + 2) == Some(b"\\r") {
      push_wrapped(&mut output, &mut column, 2, &format!("{}\\r{}", BOLD_RED, RESET));
      i += 2;
    } else if bytes.get(i..i + 2) == Some(b"\\n") {
      push_wrapped(&mut output, &mut column, 2, &format!("{}\\n{}", BOLD_RED, RESET));
      i += 2;
    } else {
      let mut token = String::new();
      token.push_str(BOLD_RED);
      token.push('\\');

      if let Some(next) = bytes.get(i + 1) {
        token.push(*next as char);
        i += 2;
      } else {
        i += 1;
      }

      token.push_str(RESET);
      push_wrapped(&mut output, &mut column, 2, &token);
    }
  }

  output
}

fn line_width(line: &str) -> usize { line.lines().map(str::len).max().unwrap_or(0) }

fn show_test(case: TestCase, section: &str) {
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

  if let Some(meta) = &case.meta {
    let should_print = match meta {
      serde_json::Value::Object(object) => !object.is_empty(),
      serde_json::Value::Null => false,
      _ => true,
    };

    if should_print {
      println!("Meta: {}", serde_json::to_string(meta).unwrap());
    }
  }

  println!("\n--- INPUT ---\n{}", highlight_input(&case.input.join("\\r\\n")));

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
                let status = headers.status.unwrap();

                Some(format!(
                  "off={} headers status=\"{}\" protocol=\"{}\" version=\"{}\" body=\"{}\"",
                  offset, status, headers.protocol, headers.version, body
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
    .map(|line| line_width(line))
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
  let inquirer = Inquirer::new();
  let tests = list_tests(section);

  let unchecked_test: Vec<TestCase> = tests.into_iter().filter(|t| !t.checked).collect();

  let mut index = 0;
  let total = unchecked_test.len();
  for test in unchecked_test {
    index += 1;
    print!("\x1bc");
    println!("--- TEST {} of {} ---", index, total);
    let path = test.path.clone();
    show_test(test, section);

    print!("Do you want to mark this test as checked? [y/N/q] ");
    std::io::stdout().flush().unwrap();

    match inquirer.query() {
      InquirerDecision::Check => {
        check_test(path);
      }
      InquirerDecision::Quit => {
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
    println!("  --generate <test>             Generate output for a test");
    println!("  --review <section>            Review all tests in a section");
    println!("  --show   <test>               Show the input and output of a test");
    println!("  --update <test>               Updates a test with the current parser");

    return;
  }

  match args.get(1).unwrap().as_str() {
    "--generate" => {
      let test = args.get(2).unwrap();
      let section = section_from_fixture_path(test).unwrap_or("unknown");
      let result = run_test(section, test);

      print!("---\n{}", result.actual);
    }
    "--review" => {
      review_tests(args.get(2).unwrap());
    }
    "--show" => {
      let test = args.get(2).unwrap();
      let section = section_from_fixture_path(test).unwrap_or("unknown");

      show_test(load_test(section, test), section);
    }
    "--update" => {
      let test = args.get(2).unwrap();
      let section = section_from_fixture_path(test).unwrap_or("unknown");

      update_test(test);
      show_test(load_test(section, test), section);
    }
    arg => {
      panic!("Unknown argument: {}", arg);
    }
  }
}
