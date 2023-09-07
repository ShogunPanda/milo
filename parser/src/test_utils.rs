use std::collections::HashMap;
use std::os::raw::c_uchar;
use std::ptr;
use std::slice;
use std::str;
use std::sync::Mutex;

use regex::Regex;

use crate::{Parser, RESPONSE};

lazy_static! {
  static ref TEST_SPANS: Mutex<HashMap<(isize, String), String>> = Mutex::new(HashMap::new());
  static ref TEST_OUTPUTS: Mutex<HashMap<isize, String>> = Mutex::new(HashMap::new());
}

static mut PARSER_COUNTER: isize = 0;

fn format_event(name: &str) -> String { format!("{}", format!("\"{}\"", name)) }

fn append_output(parser: &mut Parser, message: String, data: *const c_uchar, size: usize) -> isize {
  println!(
    "{{ {}, \"data\": {} }}",
    message,
    if data != ptr::null() {
      format!("\"{}\"", unsafe {
        str::from_utf8_unchecked(slice::from_raw_parts(data, size))
      })
    } else {
      "null".into()
    },
  );

  TEST_OUTPUTS
    .lock()
    .unwrap()
    .get_mut(&parser.values.id)
    .unwrap()
    .push_str((message + "\n").as_str());

  0
}

fn show_span(parser: &mut Parser, name: &str, data: *const c_uchar, size: usize) -> isize {
  if name == "version" || name == "protocol" || name == "method" || name == "url" {
    unsafe {
      TEST_SPANS.lock().unwrap().insert(
        (parser.values.id, name.into()),
        String::from_utf8_unchecked(slice::from_raw_parts(data, size).into()),
      );
    }
  }

  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, format_event(name)),
    data,
    size,
  )
}

fn status_complete(name: &str, parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, name),
    data,
    size,
  )
}

fn message_start(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, format_event("begin")),
    data,
    size,
  )
}

fn message_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, format_event("complete")),
    data,
    size,
  )
}

fn on_error(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  unsafe {
    let error_description = str::from_utf8_unchecked(slice::from_raw_parts(data, size));

    append_output(
      parser,
      format!(
        "\"pos\": {}, \"event\": {}, \"error code={} reason=\"{}\"",
        parser.position, "error", parser.error_code as usize, error_description
      ),
      data,
      size,
    )
  }
}

fn on_finish(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, format_event("finish")),
    data,
    size,
  )
}

fn on_request(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, format_event("request")),
    data,
    size,
  )
}

fn on_response(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, format_event("response")),
    data,
    size,
  )
}

fn on_method(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "method", data, size)
}

fn on_url(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize { show_span(parser, "url", data, size) }

fn on_protocol(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "protocol", data, size)
}

fn on_version(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "version", data, size)
}

fn on_status(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "status", data, size)
}

fn on_reason(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "reason", data, size)
}

fn on_header_name(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "header_name", data, size)
}

fn on_header_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "header_value", data, size)
}

fn on_headers(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  let spans = TEST_SPANS.lock().unwrap();

  let position = parser.position;
  let version = spans
    .get(&(parser.values.id, "version".into()))
    .unwrap()
    .replace(".", "/");
  let chunked = parser.values.has_chunked_transfer_encoding == 1;
  let content_length = parser.values.content_length;
  let protocol = spans.get(&(parser.values.id, "protocol".into())).unwrap();

  if parser.values.message_type == RESPONSE {
    let heading = format!(
      "\"pos\": {}, \"event\": {}, \"type\": \"response\", ",
      position,
      format_event("headers")
    );

    if chunked {
      append_output(
        parser,
        format!(
          "{}\"status\": \"{}\", \"protocol\": \"{}\" \"version\": \"{}\", \"body\": \"chunked\"",
          heading, parser.values.status, protocol, version,
        ),
        data,
        size,
      )
    } else if content_length > 0 {
      append_output(
        parser,
        format!(
          "{}\"status\": \"{}\", \"protocol\": \"{}\" \"version\": \"{}\", \"body\": {}\"",
          heading, parser.values.status, protocol, version, content_length
        ),
        data,
        size,
      )
    } else {
      append_output(
        parser,
        format!(
          "{}\"status\": \"{}\", \"protocol\": \"{}\" \"version\": \"{}\", \"body\": null",
          heading, parser.values.status, protocol, version,
        ),
        data,
        size,
      )
    }
  } else {
    let heading = format!(
      "\"pos\": {}, \"event\": {}, \"type\": \"response\", ",
      position,
      format_event("headers")
    );
    let method = spans.get(&(parser.values.id, "method".into())).unwrap();
    let url = spans.get(&(parser.values.id, "url".into())).unwrap();

    if chunked {
      append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\" \"protocol\": \"{}\", \"version\": \"{}\", \"body\": \"chunked\"",
          heading, method, url, protocol, version,
        ),
        data,
        size,
      )
    } else if content_length > 0 {
      append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\" \"protocol\": \"{}\", \"version\": \"{}\", \"body\": {}",
          heading, method, url, protocol, version, content_length
        ),
        data,
        size,
      )
    } else {
      append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\" \"protocol\": \"{}\", \"version\": \"{}\", \"body\": null",
          heading, method, url, protocol, version,
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
  show_span(parser, "chunk_length", data, size)
}

fn on_chunk_extension_name(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_extensions_name", data, size)
}

fn on_chunk_extension_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_extension_value", data, size)
}

fn on_chunk_data(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_data", data, size)
}

fn on_body(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize { show_span(parser, "body", data, size) }

fn on_data(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, format_event("data")),
    data,
    size,
  )
}

fn on_trailer_name(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "trailer_name", data, size)
}

fn on_trailer_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "trailer_value", data, size)
}

fn on_trailers(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
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

  parser.callbacks.on_error = on_error;
  parser.callbacks.on_finish = on_finish;
  parser.callbacks.on_request = on_request;
  parser.callbacks.on_response = on_response;
  parser.callbacks.on_message_start = message_start;
  parser.callbacks.on_message_complete = message_complete;
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
  parser.callbacks.on_chunk_data = on_chunk_data;
  parser.callbacks.on_body = on_body;
  parser.callbacks.on_data = on_data;
  parser.callbacks.on_trailer_name = on_trailer_name;
  parser.callbacks.on_trailer_value = on_trailer_value;
  parser.callbacks.on_trailers = on_trailers;

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

pub fn output(input: &str) -> String { input.trim().to_owned() + "\n" }
