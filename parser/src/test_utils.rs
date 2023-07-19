use milo::Parser;
use milo_parser_generator::{get_span, get_value};
use std::ffi::CStr;
use std::os::raw::c_char;

fn append_output(parser: &mut Parser, message: String) -> isize {
  println!("{}", message);
  parser
    .spans
    .debug
    .extend_from_slice((message + "\n").into_bytes().as_slice());
  0
}

fn show_span(name: &str, parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  append_output(
    parser,
    format!(
      "off={} len={} span[{}]=\"{}\"",
      (parser.position as isize) - (size as isize),
      size,
      name,
      ptr_to_string(data)
    ),
  )
}

fn status_complete(name: &str, parser: &mut Parser) -> isize {
  append_output(parser, format!("off={} {} complete", parser.position, name))
}

fn ptr_to_string<'a>(data: *const c_char) -> &'a str {
  unsafe { CStr::from_ptr(data).to_str().unwrap() }
}

fn message_start(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  append_output(parser, format!("off={} message begin", parser.position))
}

fn message_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  append_output(parser, format!("off={} message complete", parser.position))
}

fn on_error(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  append_output(
    parser,
    format!(
      "off={} error code={} reason=\"{}\"",
      parser.position,
      0,
      get_span!(error_reason)
    ),
  )
}

fn on_method(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_span("method", parser, data, size)
}

fn on_method_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("method", parser)
}

fn on_url(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_span("url", parser, data, size)
}

fn on_url_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("url", parser)
}

fn on_version(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_span("version", parser, data, size)
}

fn on_version_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("version", parser)
}

fn on_header_field(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_span("header_field", parser, data, size)
}

fn on_header_field_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("header_field", parser)
}

fn on_header_value(parser: &mut Parser, data: *const c_char, size: usize) -> isize {
  show_span("header_value", parser, data, size)
}

fn on_header_value_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  status_complete("header_value", parser)
}

fn on_headers_complete(parser: &mut Parser, _data: *const c_char, _size: usize) -> isize {
  append_output(
    parser,
    format!(
      "off={} headers complete method={} v={} flags=0 content_length={}",
      parser.position,
      get_span!(method),
      get_span!(version).replace(".", "/"),
      get_value!(expected_content_length)
    ),
  )
}

pub fn create_parser() -> Parser {
  let mut parser = Parser::new();

  parser.callbacks.on_error = Some(on_error);
  parser.callbacks.on_message_start = Some(message_start);
  parser.callbacks.on_message_complete = Some(message_complete);
  parser.callbacks.on_method = Some(on_method);
  parser.callbacks.on_method_complete = Some(on_method_complete);
  parser.callbacks.on_url = Some(on_url);
  parser.callbacks.on_url_complete = Some(on_url_complete);
  parser.callbacks.on_version = Some(on_version);
  parser.callbacks.on_version_complete = Some(on_version_complete);
  parser.callbacks.on_header_field = Some(on_header_field);
  parser.callbacks.on_header_field_complete = Some(on_header_field_complete);
  parser.callbacks.on_header_value = Some(on_header_value);
  parser.callbacks.on_header_value_complete = Some(on_header_value_complete);
  parser.callbacks.on_headers_complete = Some(on_headers_complete);

  parser
}

pub fn http(input: &str) -> String {
  input.trim().replace("\n", "\r\n") + "\r\n\r\n"
}

#[allow(dead_code)]
pub fn output(input: &str) -> String {
  input.trim().to_owned() + "\n"
}
