use core::{slice, str};
use std::collections::HashSet;
use std::ffi::c_void;
use std::fs::read_to_string;
use std::str::from_utf8_unchecked;
use std::vec;
use std::{fs::read_dir, path::Path};

use milo::{CALLBACK_ACTIVE_ALL, MESSAGE_TYPE_REQUEST, MESSAGE_TYPE_RESPONSE, Parser};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::helpers::output::extract_payload;

#[derive(Debug)]
struct Context {
  pub input: String,
  pub status: u32,
  pub method: String,
  pub reason: String,
  pub url: String,
  pub protocol: String,
  pub version: String,
  pub events: Vec<Event>,
}

impl Context {
  pub fn new() -> Self {
    Context {
      input: String::new(),
      status: 0,
      method: String::new(),
      reason: String::new(),
      url: String::new(),
      protocol: String::new(),
      version: String::new(),
      events: Vec::new(),
    }
  }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Body {
  String(String),
  Number(u32),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Headers {
  pub method: Option<String>,
  pub status: Option<u32>,
  pub url: Option<String>,
  pub protocol: String,
  pub version: String,
  pub body: Option<Body>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Error {
  pub code: String,
  pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Payload {
  String(String),
  Headers(Headers),
  Error(Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
  pub offset: usize,
  #[serde(rename = "type")]
  pub kind: String,
  pub payload: Option<Payload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
  pub path: String,
  pub line: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TestCase {
  pub path: String,
  pub name: String,
  pub checked: bool,
  pub source: Source,
  pub input: Vec<String>,
  pub llhttp: Vec<String>,
  pub output: Option<Vec<Event>>,
}

#[allow(unused)]
pub struct Result {
  original_input: Vec<String>,
  normalized_input: String,
  expected: String,
  pub actual: String,
  parsed: usize,
  state: String,
  error_code: String,
  error_description: String,
}

fn js_from_char_code(value: u32) -> String {
  let unit = (value & 0xFFFF) as u16;
  String::from_utf16_lossy(&[unit])
}

fn add_event(parser: &mut Parser, kind: &str, from: usize, size: usize) {
  let mut context = unsafe { Box::from_raw(parser.context as *mut Context) };

  let mut payload = None;
  if size > 0 {
    let (data, cleanup) = extract_payload(parser, from, size);

    let payload_str = unsafe { from_utf8_unchecked(slice::from_raw_parts(data, size)).to_string() };
    payload = Some(Payload::String(payload_str.clone()));

    match kind {
      "method" => {
        context.method = payload_str;
      }
      "url" => {
        context.url = payload_str;
      }
      "protocol" => {
        context.protocol = payload_str;
      }
      "version" => {
        context.version = payload_str;
      }
      "status" => {
        context.status = payload_str.parse().unwrap_or(0);
      }
      "reason" => {
        context.reason = payload_str;
      }
      _ => {}
    }

    cleanup();
  }

  context.events.push(Event {
    offset: from,
    kind: kind.to_string(),
    payload: payload,
  });

  let _ = Box::into_raw(context);
}

fn on_error(parser: &mut Parser, from: usize, _size: usize) {
  let mut context = unsafe { Box::from_raw(parser.context as *mut Context) };

  context.events.push(Event {
    offset: from,
    kind: "error".into(),
    payload: Some(Payload::Error(Error {
      code: parser.error_code_str().to_string(),
      description: parser.error_description_str().to_string(),
    })),
  });

  let _ = Box::into_raw(context);
}

fn on_finish(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "finish", from, size); }

fn on_request(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "request", from, size); }

fn on_response(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "response", from, size); }

fn on_message_start(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "begin", from, size); }

fn on_message_complete(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "complete", from, size); }

fn on_method(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "method", from, size); }

fn on_url(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "url", from, size); }

fn on_protocol(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "protocol", from, size); }

fn on_version(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "version", from, size); }

fn on_status(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "status", from, size); }

fn on_reason(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "reason", from, size); }

fn on_header_name(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "header_name", from, size); }

fn on_header_value(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "header_value", from, size); }

fn on_headers(parser: &mut Parser, from: usize, _size: usize) {
  let mut context = unsafe { Box::from_raw(parser.context as *mut Context) };

  let body = if parser.has_content_length {
    Some(Body::Number(parser.content_length as u32))
  } else if parser.has_chunked_transfer_encoding {
    Some(Body::String("chunked".into()))
  } else {
    None
  };

  let payload = if parser.message_type == MESSAGE_TYPE_REQUEST {
    Some(Payload::Headers(Headers {
      method: Some(context.method.clone()),
      url: Some(context.url.clone()),
      status: None,
      protocol: context.protocol.clone(),
      version: context.version.clone(),
      body: body,
    }))
  } else {
    Some(Payload::Headers(Headers {
      method: None,
      url: None,
      status: Some(context.status),
      protocol: context.protocol.clone(),
      version: context.version.clone(),
      body: body,
    }))
  };

  context.events.push(Event {
    offset: from,
    kind: "headers".into(),
    payload: payload,
  });

  let _ = Box::into_raw(context);
}

fn on_upgrade(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "upgrade", from, size); }

fn on_chunk_length(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "chunk_length", from, size); }

fn on_chunk_extension_name(parser: &mut Parser, from: usize, size: usize) {
  add_event(parser, "chunk_extension_name", from, size);
}

fn on_chunk_extension_value(parser: &mut Parser, from: usize, size: usize) {
  add_event(parser, "chunk_extension_value", from, size);
}

fn on_chunk(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "chunk", from, size); }

fn on_body(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "body", from, size); }

fn on_data(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "data", from, size); }

fn on_trailer_name(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "trailer_name", from, size); }

fn on_trailer_value(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "trailer_value", from, size); }

fn on_trailers(parser: &mut Parser, from: usize, size: usize) { add_event(parser, "trailers", from, size); }

pub fn parse_input(raw: &Vec<String>) -> String {
  let mut input = raw.join("\n");

  // Remove escaped physical newlines: "\" + CRLF/CR/LF
  input = Regex::new(r"\\(?:\r\n|\r|\n)")
    .unwrap()
    .replace_all(&input, "")
    .to_string();

  // Normalize all physical newlines to CRLF
  input = Regex::new(r"(?:\r\n|\r|\n)")
    .unwrap()
    .replace_all(&input, "\r\n")
    .to_string();

  // Replace escaped sequences

  input = Regex::new(r"\\(?:n|r|t|f|x([0-9A-Fa-f]+)|([0-7]{1,3}))")
    .unwrap()
    .replace_all(&input, |caps: &regex::Captures| {
      let whole = caps.get(0).unwrap().as_str();

      match whole {
        "\\n" => "\n".to_string(),
        "\\r" => "\r".to_string(),
        "\\t" => "\t".to_string(),
        "\\f" => "\x0C".to_string(),
        _ => {
          if let Some(hex) = caps.get(1) {
            if let Ok(v) = u32::from_str_radix(hex.as_str(), 16) {
              return js_from_char_code(v);
            } else {
              return whole.to_string();
            }
          }

          if let Some(oct) = caps.get(2) {
            if let Ok(v) = u32::from_str_radix(oct.as_str(), 8) {
              return js_from_char_code(v);
            } else {
              return whole.to_string();
            }
          }

          whole.to_string()
        }
      }
    })
    .to_string();

  input
}

pub fn create_parser(input: String) -> Parser {
  let mut parser = Parser::new();

  let mut context = Box::new(Context::new());
  context.input = input;
  parser.context = Box::into_raw(context) as *mut c_void;

  parser.callbacks.on_error = on_error;
  parser.callbacks.on_finish = on_finish;
  parser.callbacks.on_request = on_request;
  parser.callbacks.on_response = on_response;
  parser.callbacks.on_message_start = on_message_start;
  parser.callbacks.on_message_complete = on_message_complete;
  parser.callbacks.on_method = on_method;
  parser.callbacks.on_url = on_url;
  parser.callbacks.on_protocol = on_protocol;
  parser.callbacks.on_version = on_version;
  parser.callbacks.on_status = on_status;
  parser.callbacks.on_reason = on_reason;
  parser.callbacks.on_header_name = on_header_name;
  parser.callbacks.on_header_value = on_header_value;
  parser.callbacks.on_headers = on_headers;
  parser.callbacks.on_upgrade = on_upgrade;
  parser.callbacks.on_chunk_length = on_chunk_length;
  parser.callbacks.on_chunk_extension_name = on_chunk_extension_name;
  parser.callbacks.on_chunk_extension_value = on_chunk_extension_value;
  parser.callbacks.on_chunk = on_chunk;
  parser.callbacks.on_body = on_body;
  parser.callbacks.on_data = on_data;
  parser.callbacks.on_trailer_name = on_trailer_name;
  parser.callbacks.on_trailer_value = on_trailer_value;
  parser.callbacks.on_trailers = on_trailers;
  parser.active_callbacks = CALLBACK_ACTIVE_ALL;

  parser
}

#[allow(unused)]
pub fn list_tests(section: &str) -> Vec<TestCase> {
  let files = read_dir(Path::new(&format!("./tests/fixtures/llhttp/{}", section))).unwrap();

  files
    .into_iter()
    .map(|file| {
      let file = file.unwrap().path();
      let path = file.to_str().unwrap().to_string();
      let raw = read_to_string(&path).unwrap();
      let mut case: TestCase = serde_yaml::from_str(&raw).unwrap();

      case.path = path;
      case
    })
    .collect()
}

#[allow(unused)]
pub fn load_test(section: &str, path: &str) -> TestCase {
  let raw = read_to_string(path).unwrap();

  // Unwrap the test case and normalize the input
  serde_yaml::from_str(&raw).unwrap()
}

#[allow(unused)]
pub fn run_test(section: &str, path: &str) -> Result {
  let raw = read_to_string(path).unwrap();

  // Unwrap the test case and normalize the input
  let case: TestCase = serde_yaml::from_str(&raw).unwrap();
  let input = parse_input(&case.input);

  // Create the parser with its context
  let mut parser = create_parser(input.clone());
  if section == "requests" {
    parser.mode = MESSAGE_TYPE_REQUEST;
  } else {
    parser.mode = MESSAGE_TYPE_RESPONSE;
  }

  // Perform the parsing
  let len = parser.parse(input.as_ptr(), input.len());

  // Compare output
  let mut context = unsafe { Box::from_raw(parser.context as *mut Context) };
  let actual = serde_yaml::to_string(&context.events).unwrap();
  let expected = serde_yaml::to_string(&case.output).unwrap();

  let result = Result {
    original_input: case.input,
    normalized_input: input,
    expected,
    actual,
    parsed: len,
    state: parser.state_str().into(),
    error_code: parser.error_code_str().into(),
    error_description: parser.error_description_str().into(),
  };

  // Memory cleanup
  let _ = Box::into_raw(context);

  result
}

#[allow(unused)]
pub fn run_tests(section: &str, only: &Option<HashSet<String>>) {
  let files = read_dir(Path::new(&format!("./tests/fixtures/llhttp/{}", section))).unwrap();

  let mut unchecked = vec![];

  for file in files {
    // Mangle the path
    let file = file.unwrap().path();
    let path = file.to_str().unwrap().to_string();
    let raw = read_to_string(&path).unwrap();

    // Unwrap the test case and normalize the input
    let case: TestCase = serde_yaml::from_str(&raw).unwrap();

    if case.checked {
      eprintln!("✅ {}", path);
    } else {
      eprintln!("❌ {}", path);
      unchecked.push(path);
    }
  }

  assert!(unchecked.is_empty(), "Detected unchecked tests:\n\n{:#?}", unchecked);
}
