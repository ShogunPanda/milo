use crate::{Parser, RESPONSE};
use milo_parser_generator::{get_span, get_value};
use regex::Regex;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;

lazy_static! {
  static ref TEST_SPANS: Mutex<HashMap<(isize, String), String>> = Mutex::new(HashMap::new());
  static ref TEST_OUTPUTS: Mutex<HashMap<isize, String>> = Mutex::new(HashMap::new());
}

static mut PARSER_COUNTER: isize = 0;

fn append_output(parser: &mut Parser, message: String) -> isize {
  println!("{}", message);

  TEST_OUTPUTS
    .lock()
    .unwrap()
    .get_mut(&parser.values.id)
    .unwrap()
    .push_str((message + "\n").as_str());

  0
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn ptr_to_string<'a>(data: *const c_char) -> &'a str {
  unsafe { CStr::from_ptr(data).to_str().unwrap() }
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn show_data(name: &str, parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  append_output(
    parser,
    format!(
      "off={} len={} data[{}]=\"{}\"",
      parser.position,
      size,
      name,
      ptr_to_string(data)
    ),
  )
}

fn show_span(parser: &mut Parser, name: &str, value: String) -> isize {
  append_output(parser, format!("off={} span[{}]=\"{}\"", parser.position, name, value))
}

fn status_complete(name: &str, parser: &mut Parser) -> isize {
  append_output(parser, format!("off={} {} complete", parser.position, name))
}

fn message_start(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  append_output(parser, format!("off={} message begin", parser.position))
}

fn message_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  append_output(parser, format!("off={} message complete", parser.position))
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_method(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("method", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_url(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("url", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_protocol(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("protocol", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_version(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("version", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_header_field(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("header_field", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_header_value(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("header_value", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_length(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("chunk_length", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_extension_name(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("chunk_extension_name", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_extension_value(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("chunk_extension_value", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_data(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_data("chunk_data", parser, data, size)
}

fn on_error(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  append_output(
    parser,
    format!(
      "off={} error code={} reason=\"{}\"",
      parser.position, parser.error_code as usize, parser.error_str
    ),
  )
}

fn on_method(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "method", get_span!(method))
}

fn on_method_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("method", parser)
}

fn on_url(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "url", get_span!(url))
}

fn on_url_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("url", parser)
}

fn on_protocol(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "protocol", get_span!(protocol))
}

fn on_protocol_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("protocol", parser)
}

fn on_version(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "version", get_span!(version))
}

fn on_version_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("version", parser)
}

fn on_header_field(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "header_field", get_span!(header_field))
}

fn on_header_field_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("header_field", parser)
}

fn on_header_value(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "header_value", get_span!(header_value))
}

fn on_header_value_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("header_value", parser)
}

fn on_chunk_length(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "chunk_length", get_span!(chunk_length))
}

fn on_chunk_extension_name(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "chunk_extensions_name", get_span!(chunk_extension_name))
}

fn on_chunk_extension_value(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "chunk_extension_value", get_span!(chunk_extension_value))
}

fn on_chunk_data(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  show_span(parser, "chunk_data", get_span!(chunk_data))
}

fn on_headers_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  let position = parser.position;
  let version = get_span!(version).replace(".", "/");
  let content_length = get_value!(expected_content_length);
  let protocol = get_span!(protocol);

  if parser.values.message_type == RESPONSE {
    append_output(
      parser,
      format!(
        "off={} headers complete type=response status={} protocol={} v={} content_length={}",
        position, parser.values.response_status, protocol, version, content_length
      ),
    )
  } else {
    let method = get_span!(method);
    let url = get_span!(url);

    append_output(
      parser,
      format!(
        "off={} headers complete type=request method={} url={} protocol={} v={} content_length={}",
        position, method, url, protocol, version, content_length
      ),
    )
  }
}

pub fn create_parser() -> Parser {
  let id = unsafe {
    PARSER_COUNTER += 1;
    PARSER_COUNTER
  };
  let mut parser = Parser::new();
  parser.values.id = id;

  let mut outputs = TEST_OUTPUTS.lock().unwrap();
  let mut spans = TEST_SPANS.lock().unwrap();

  outputs.insert(id, String::new());
  spans.insert((id, "method".into()), String::new());
  spans.insert((id, "url".into()), String::new());
  spans.insert((id, "protocol".into()), String::new());
  spans.insert((id, "version".into()), String::new());
  spans.insert((id, "header_field".into()), String::new());
  spans.insert((id, "header_value".into()), String::new());
  spans.insert((id, "chunk_length".into()), String::new());
  spans.insert((id, "chunk_extension_name".into()), String::new());
  spans.insert((id, "chunk_extension_value".into()), String::new());
  spans.insert((id, "chunk_data".into()), String::new());

  parser.callbacks.on_error = Some(on_error);
  parser.callbacks.on_message_start = Some(message_start);
  parser.callbacks.on_message_complete = Some(message_complete);

  #[cfg(all(debug_assertions, feature = "milo_debug_test"))]
  {
    parser.callbacks.on_data_method = Some(on_data_method);
    parser.callbacks.on_data_url = Some(on_data_url);
    parser.callbacks.on_data_protocol = Some(on_data_protocol);
    parser.callbacks.on_data_version = Some(on_data_version);
    parser.callbacks.on_data_header_field = Some(on_data_header_field);
    parser.callbacks.on_data_header_value = Some(on_data_header_value);
    parser.callbacks.on_data_chunk_length = Some(on_data_chunk_length);
    parser.callbacks.on_data_chunk_extension_name = Some(on_data_chunk_extension_name);
    parser.callbacks.on_data_chunk_extension_value = Some(on_data_chunk_extension_value);
    parser.callbacks.on_data_chunk_data = Some(on_data_chunk_data);
  }

  parser.callbacks.on_method = Some(on_method);
  parser.callbacks.on_method_complete = Some(on_method_complete);
  parser.callbacks.on_url = Some(on_url);
  parser.callbacks.on_url_complete = Some(on_url_complete);
  parser.callbacks.on_protocol = Some(on_protocol);
  parser.callbacks.on_protocol_complete = Some(on_protocol_complete);
  parser.callbacks.on_version = Some(on_version);
  parser.callbacks.on_version_complete = Some(on_version_complete);
  parser.callbacks.on_header_field = Some(on_header_field);
  parser.callbacks.on_header_field_complete = Some(on_header_field_complete);
  parser.callbacks.on_header_value = Some(on_header_value);
  parser.callbacks.on_header_value_complete = Some(on_header_value_complete);
  parser.callbacks.on_headers_complete = Some(on_headers_complete);
  parser.callbacks.on_chunk_length = Some(on_chunk_length);
  parser.callbacks.on_chunk_extension_name = Some(on_chunk_extension_name);
  parser.callbacks.on_chunk_extension_value = Some(on_chunk_extension_value);
  parser.callbacks.on_chunk_data = Some(on_chunk_data);

  parser
}

pub fn length(input: *mut i8) -> usize {
  let str = unsafe { CStr::from_ptr(input).to_str().unwrap() };
  str.len()
}

pub fn http(input: &str) -> *mut i8 {
  let trailing_ws = Regex::new(r"(?m)^\s+").unwrap();
  let sanitized = trailing_ws
    .replace_all(input.trim(), "")
    .replace("\n", "")
    .replace("\\r", "\r")
    .replace("\\n", "\n")
    .replace("\\s", " ");

  CString::new(sanitized.to_string()).unwrap().into_raw()
}

pub fn output(input: &str) -> String {
  input.trim().to_owned() + "\n"
}
