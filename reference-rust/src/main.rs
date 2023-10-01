#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::os::raw::c_uchar;
use std::slice;
use std::str;
use std::sync::Mutex;

use milo::{Parser, RESPONSE};

lazy_static! {
  static ref TEST_SPANS: Mutex<HashMap<(isize, String), String>> = Mutex::new(HashMap::new());
}

fn format_event(name: &str) -> String { format!("{}", format!("\"{}\"", name)) }

fn append_output(_parser: &mut Parser, message: String, data: *const c_uchar, size: usize) -> isize {
  println!(
    "{{ {}, \"data\": {} }}",
    message,
    if !data.is_null() {
      format!("\"{}\"", unsafe {
        str::from_utf8_unchecked(slice::from_raw_parts(data, size))
      })
    } else {
      "null".into()
    },
  );

  0
}

fn event(parser: &mut Parser, name: &str, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!("\"pos\": {}, \"event\": {}", parser.position, name),
    data,
    size,
  )
}

fn show_span(parser: &mut Parser, name: &str, data: *const c_uchar, size: usize) -> isize {
  if name == "version" || name == "protocol" || name == "method" || name == "url" {
    unsafe {
      TEST_SPANS.lock().unwrap().insert(
        (parser.id, name.into()),
        String::from_utf8_unchecked(slice::from_raw_parts(data, size).into()),
      );
    }
  }

  event(parser, name, data, size)
}

fn before_state_change(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"before_state_change\", \"current_state\": \"{}\"",
      parser.position,
      parser.state_string()
    ),
    data,
    size,
  )
}

fn after_state_change(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"after_state_change\", \"current_state\": \"{}\"",
      parser.position,
      parser.state_string()
    ),
    data,
    size,
  )
}

fn on_message_start(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  event(parser, "begin", data, size)
}

fn on_message_complete(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  event(parser, "complete", data, size)
}

fn on_error(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  unsafe {
    append_output(
      parser,
      format!(
        "\"pos\": {}, \"event\": {}, \"error_code={}, \"error_code_string\": \"{}\", reason=\"{}\"",
        parser.position,
        "error",
        parser.error_code as usize,
        parser.error_code_string(),
        str::from_utf8_unchecked(slice::from_raw_parts(data, size))
      ),
      data,
      size,
    )
  }
}

fn on_finish(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize { event(parser, "finish", data, size) }

fn on_request(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize { event(parser, "request", data, size) }

fn on_response(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  event(parser, "response", data, size)
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
  let version = spans.get(&(parser.id, "version".into())).unwrap().replace('.', "/");
  let chunked = parser.has_chunked_transfer_encoding == 1;
  let content_length = parser.content_length;
  let protocol = spans.get(&(parser.id, "protocol".into())).unwrap();

  if parser.message_type == RESPONSE {
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
          heading, parser.status, protocol, version,
        ),
        data,
        size,
      )
    } else if content_length > 0 {
      append_output(
        parser,
        format!(
          "{}\"status\": \"{}\", \"protocol\": \"{}\" \"version\": \"{}\", \"body\": {}\"",
          heading, parser.status, protocol, version, content_length
        ),
        data,
        size,
      )
    } else {
      append_output(
        parser,
        format!(
          "{}\"status\": \"{}\", \"protocol\": \"{}\" \"version\": \"{}\", \"body\": null",
          heading, parser.status, protocol, version,
        ),
        data,
        size,
      )
    }
  } else {
    let heading = format!(
      "\"pos\": {}, \"event\": {}, \"type\": \"request\", ",
      position,
      format_event("headers")
    );
    let method = spans.get(&(parser.id, "method".into())).unwrap();
    let url = spans.get(&(parser.id, "url".into())).unwrap();

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

fn on_upgrade(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize { event(parser, "upgrade", data, size) }

fn on_chunk_length(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_length", data, size)
}

fn on_chunk_extension_name(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_extensions_name", data, size)
}

fn on_chunk_extension_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "chunk_extension_value", data, size)
}

fn on_body(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize { event(parser, "body", data, size) }

fn on_data(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize { show_span(parser, "data", data, size) }

fn on_trailer_name(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "trailer_name", data, size)
}

fn on_trailer_value(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  show_span(parser, "trailer_value", data, size)
}

fn on_trailers(parser: &mut Parser, data: *const c_uchar, size: usize) -> isize {
  event(parser, "trailers", data, size)
}

fn main() {
  let mut parser = Parser::new();

  let request1 = "GET / HTTP/1.1\r\n\r\n";
  let request2 = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nTrailer: x-trailer\r\n\r\nc;need=love\r\nhello \
                  world!\r\n0\r\nX-Trailer: value\r\n\r\n";

  parser.callbacks.before_state_change = before_state_change;
  parser.callbacks.after_state_change = after_state_change;
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
  parser.callbacks.on_body = on_body;
  parser.callbacks.on_data = on_data;
  parser.callbacks.on_trailer_name = on_trailer_name;
  parser.callbacks.on_trailer_value = on_trailer_value;
  parser.callbacks.on_trailers = on_trailers;

  let mut consumed = unsafe { parser.parse(request1.as_ptr(), request1.len()) };

  println!(
    "{{ \"pos\": {}, \"consumed\": {}, \"state\": \"{}\" }}",
    parser.position,
    consumed,
    parser.state_string()
  );

  println!("------------------------------------------------------------------------------------------\n");

  consumed = unsafe { parser.parse(request2.as_ptr(), request2.len()) };
  println!(
    "{{ \"pos\": {}, \"consumed\": {}, \"state\": \"{}\" }}",
    parser.position,
    consumed,
    parser.state_string()
  );
}
