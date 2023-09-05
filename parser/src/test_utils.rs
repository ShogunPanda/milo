use regex::Regex;
use std::collections::HashMap;
use std::os::raw::c_uchar;
use std::ptr;
use std::slice;
use std::str;
use std::sync::Mutex;

use crate::{Parser, RESPONSE};

lazy_static! {
  static ref TEST_SPANS: Mutex<HashMap<(isize, String), String>> = Mutex::new(HashMap::new());
  static ref TEST_OUTPUTS: Mutex<HashMap<isize, String>> = Mutex::new(HashMap::new());
}

macro_rules! get_span {
  ($parser:ident, $field:ident) => {
    unsafe { format!("{}", str::from_utf8_unchecked(&$parser.spans.$field[..])) }
  };
}

static mut PARSER_COUNTER: isize = 0;

fn append_output(parser: &mut Parser, message: String, data: *const c_uchar, size: usize) -> isize {
  println!(
    "{} | cb_len={} cb_data=\"{}\"",
    message,
    size,
    if data != ptr::null() {
      unsafe { str::from_utf8_unchecked(slice::from_raw_parts(data, size)) }
    } else {
      "NULL"
    }
  );

  TEST_OUTPUTS
    .lock()
    .unwrap()
    .get_mut(&parser.values.id)
    .unwrap()
    .push_str((message + "\n").as_str());

  0
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn show_data(name: &str, parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!(
      "pos={} data[{}]=\"{}\" (len={})",
      parser.position,
      name,
      parser.get_span(data),
      size,
    ),
    data,
    size,
  )
}

fn show_span(parser: &mut Parser, name: &str, value: String, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("pos={} span[{}]=\"{}\"", parser.position, name, value),
    data,
    size,
  )
}

fn status_complete(name: &str, parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(parser, format!("pos={} {} complete", parser.position, name), data, size)
}

fn message_start(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(parser, format!("pos={} message begin", parser.position), data, size)
}

fn message_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(parser, format!("pos={} message complete", parser.position), data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_method(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("method", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_url(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("url", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_protocol(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("protocol", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_version(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("version", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_header_field(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("header_field", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_header_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("header_value", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_length(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("chunk_length", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_extension_name(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("chunk_extension_name", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_extension_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("chunk_extension_value", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_chunk_data(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("chunk_data", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_body(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("body", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_trailer_field(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("trailer_field", parser, data, size)
}

#[cfg(all(debug_assertions, feature = "milo_debug_test"))]
fn on_data_trailer_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_data("trailer_value", parser, data, size)
}

fn on_error(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  unsafe {
    let error_description = str::from_utf8_unchecked(slice::from_raw_parts(data, size));

    append_output(
      parser,
      format!(
        "pos={} error code={} reason=\"{}\"",
        parser.position, parser.error_code as usize, error_description
      ),
      data,
      size,
    )
  }
}

fn on_finish(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(parser, format!("pos={} finish", parser.position), data, size)
}

fn on_request(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(parser, format!("pos={} request", parser.position), data, size)
}

fn on_response(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(parser, format!("pos={} response", parser.position), data, size)
}

fn on_method(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "method", get_span!(parser, method), data, size)
}

fn on_method_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("method", parser, data, size)
}

fn on_url(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "url", get_span!(parser, url), data, size)
}

fn on_url_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("url", parser, data, size)
}

fn on_protocol(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "protocol", get_span!(parser, protocol), data, size)
}

fn on_protocol_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("protocol", parser, data, size)
}

fn on_version(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "version", get_span!(parser, version), data, size)
}

fn on_version_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("version", parser, data, size)
}

fn on_status(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "status", get_span!(parser, version), data, size)
}

fn on_status_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("status", parser, data, size)
}

fn on_reason(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "reason", get_span!(parser, version), data, size)
}

fn on_reason_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("reason", parser, data, size)
}

fn on_header_field(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "header_field", get_span!(parser, header_field), data, size)
}

fn on_header_field_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("header_field", parser, data, size)
}

fn on_header_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "header_value", get_span!(parser, header_value), data, size)
}

fn on_header_value_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("header_value", parser, data, size)
}

fn on_headers_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  let position = parser.position;
  let version = get_span!(parser, version).replace(".", "/");
  let chunked = parser.values.has_chunked_transfer_encoding == 1;
  let content_length = parser.values.expected_content_length;
  let protocol = get_span!(parser, protocol);

  if parser.values.message_type == RESPONSE {
    if chunked {
      append_output(
        parser,
        format!(
          "pos={} headers complete type=response status={} protocol={} v={} chunked",
          position, parser.values.status, protocol, version,
        ),
        data,
        size,
      )
    } else if content_length > 0 {
      append_output(
        parser,
        format!(
          "pos={} headers complete type=response status={} protocol={} v={} content_length={}",
          position, parser.values.status, protocol, version, content_length
        ),
        data,
        size,
      )
    } else {
      append_output(
        parser,
        format!(
          "pos={} headers complete type=response status={} protocol={} v={} no-body",
          position, parser.values.status, protocol, version,
        ),
        data,
        size,
      )
    }
  } else {
    let method = get_span!(parser, method);
    let url = get_span!(parser, url);

    if chunked {
      append_output(
        parser,
        format!(
          "pos={} headers complete type=request method={} url={} protocol={} v={} chunked",
          position, method, url, protocol, version,
        ),
        data,
        size,
      )
    } else if content_length > 0 {
      append_output(
        parser,
        format!(
          "pos={} headers complete type=request method={} url={} protocol={} v={} content_length={}",
          position, method, url, protocol, version, content_length
        ),
        data,
        size,
      )
    } else {
      append_output(
        parser,
        format!(
          "pos={} headers complete type=request method={} url={} protocol={} v={} no-body",
          position, method, url, protocol, version,
        ),
        data,
        size,
      )
    }
  }
}

fn on_upgrade(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("upgrade", parser, data, size)
}

fn on_chunk_length(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_length", get_span!(parser, chunk_length), data, size)
}

fn on_chunk_extension_name(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(
    parser,
    "chunk_extensions_name",
    get_span!(parser, chunk_extension_name),
    data,
    size,
  )
}

fn on_chunk_extension_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(
    parser,
    "chunk_extension_value",
    get_span!(parser, chunk_extension_value),
    data,
    size,
  )
}

fn on_chunk_data(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_data", get_span!(parser, chunk_data), data, size)
}

fn on_body(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "body", get_span!(parser, body), data, size)
}

fn on_trailer_field(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "trailer_field", get_span!(parser, trailer_field), data, size)
}

fn on_trailer_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "trailer_value", get_span!(parser, trailer_value), data, size)
}

fn on_trailers_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  status_complete("trailers", parser, data, size)
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

  #[cfg(all(debug_assertions, feature = "milo_debug_test"))]
  {
    parser.callbacks.on_data_method = on_data_method;
    parser.callbacks.on_data_url = on_data_url;
    parser.callbacks.on_data_protocol = on_data_protocol;
    parser.callbacks.on_data_version = on_data_version;
    parser.callbacks.on_data_header_field = on_data_header_field;
    parser.callbacks.on_data_header_value = on_data_header_value;
    parser.callbacks.on_data_chunk_length = on_data_chunk_length;
    parser.callbacks.on_data_chunk_extension_name = on_data_chunk_extension_name;
    parser.callbacks.on_data_chunk_extension_value = on_data_chunk_extension_value;
    parser.callbacks.on_data_chunk_data = on_data_chunk_data;
    parser.callbacks.on_data_body = on_data_body;
    parser.callbacks.on_data_trailer_field = on_data_trailer_field;
    parser.callbacks.on_data_trailer_value = on_data_trailer_value;
  }

  parser.callbacks.on_error = on_error;
  parser.callbacks.on_finish = on_finish;
  parser.callbacks.on_request = on_request;
  parser.callbacks.on_response = on_response;
  parser.callbacks.on_message_start = message_start;
  parser.callbacks.on_message_complete = message_complete;
  parser.callbacks.on_method = on_method;
  parser.callbacks.on_method_complete = on_method_complete;
  parser.callbacks.on_url = on_url;
  parser.callbacks.on_url_complete = on_url_complete;
  parser.callbacks.on_protocol = on_protocol;
  parser.callbacks.on_protocol_complete = on_protocol_complete;
  parser.callbacks.on_version = on_version;
  parser.callbacks.on_version_complete = on_version_complete;
  parser.callbacks.on_status = on_status;
  parser.callbacks.on_status_complete = on_status_complete;
  parser.callbacks.on_reason = on_reason;
  parser.callbacks.on_reason_complete = on_reason_complete;
  parser.callbacks.on_header_field = on_header_field;
  parser.callbacks.on_header_field_complete = on_header_field_complete;
  parser.callbacks.on_header_value = on_header_value;
  parser.callbacks.on_header_value_complete = on_header_value_complete;
  parser.callbacks.on_headers_complete = on_headers_complete;
  parser.callbacks.on_upgrade = on_upgrade;
  parser.callbacks.on_chunk_length = on_chunk_length;
  parser.callbacks.on_chunk_extension_name = on_chunk_extension_name;
  parser.callbacks.on_chunk_extension_value = on_chunk_extension_value;
  parser.callbacks.on_chunk_data = on_chunk_data;
  parser.callbacks.on_body = on_body;
  parser.callbacks.on_trailer_field = on_trailer_field;
  parser.callbacks.on_trailer_value = on_trailer_value;
  parser.callbacks.on_trailers_complete = on_trailers_complete;

  parser
}

pub fn http(input: &str) -> String {
  let trailing_ws = Regex::new(r"(?m)^\s+").unwrap();

  trailing_ws
    .replace_all(input.trim(), "")
    .replace("\n", "")
    .replace("\\r", "\r")
    .replace("\\n", "\n")
    .replace("\\s", " ")
}

pub fn output(input: &str) -> String {
  input.trim().to_owned() + "\n"
}
