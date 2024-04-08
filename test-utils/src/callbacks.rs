#![allow(unused_variables, unreachable_code, unused_imports)]

use std::{os::unix::process, slice, str};

use milo::{Parser, DEBUG, MESSAGE_TYPE_RESPONSE};

use crate::{context, output};

pub fn on_state_change(parser: &mut Parser, from: usize, size: usize) {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"state\", \"state\": \"{}\"",
      parser.position,
      parser.state_str()
    ),
    from,
    size,
  );
}

pub fn on_message_start(parser: &mut Parser, from: usize, size: usize) {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"begin\", \"configuration\": {{ \"debug\": {} }}",
      parser.position, DEBUG,
    ),
    from,
    size,
  );
}

pub fn on_message_complete(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "complete", parser.position, from, size);
}

pub fn on_error(parser: &mut Parser, from: usize, size: usize) {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": {}, \"error_code={}, \"error_code_string\": \"{}\", reason=\"{}\"",
      parser.position,
      "error",
      parser.error_code,
      parser.error_code_str(),
      parser.error_description_str(),
    ),
    from,
    size,
  );
}

pub fn on_finish(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "finish", parser.position, from, size);
}

pub fn on_request(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "request", parser.position, from, size);
}

pub fn on_response(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "response", parser.position, from, size);
}

pub fn on_method(parser: &mut Parser, from: usize, size: usize) { output::show_span(parser, "method", from, size); }

pub fn on_url(parser: &mut Parser, from: usize, size: usize) { output::show_span(parser, "url", from, size); }

pub fn on_protocol(parser: &mut Parser, from: usize, size: usize) { output::show_span(parser, "protocol", from, size); }

pub fn on_version(parser: &mut Parser, from: usize, size: usize) { output::show_span(parser, "version", from, size); }

pub fn on_status(parser: &mut Parser, from: usize, size: usize) { output::show_span(parser, "status", from, size); }

pub fn on_reason(parser: &mut Parser, from: usize, size: usize) { output::show_span(parser, "reason", from, size); }

pub fn on_header_name(parser: &mut Parser, from: usize, size: usize) {
  output::show_span(parser, "header_name", from, size);
}

pub fn on_header_value(parser: &mut Parser, from: usize, size: usize) {
  output::show_span(parser, "header_value", from, size);
}

pub fn on_headers(parser: &mut Parser, from: usize, size: usize) {
  let context = unsafe { Box::from_raw(parser.context as *mut context::Context) };

  let position = parser.position;
  let chunked = parser.has_chunked_transfer_encoding;
  let content_length = parser.content_length;

  let method: String = context.method.clone();
  let url: String = context.url.clone();
  let protocol: String = context.protocol.clone();
  let version: String = context.version.clone();
  Box::into_raw(context);

  if parser.message_type == MESSAGE_TYPE_RESPONSE {
    let heading = format!(
      "\"pos\": {}, \"event\": {}, \"type\": \"response\", ",
      position,
      output::format_event("headers")
    );

    if chunked {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": \"chunked\"",
          heading, parser.status, protocol, version,
        ),
        from,
        size,
      );
    } else if content_length > 0 {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": {}",
          heading, parser.status, protocol, version, content_length
        ),
        from,
        size,
      );
    } else {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": null",
          heading, parser.status, protocol, version,
        ),
        from,
        size,
      );
    }
  } else {
    let heading = format!(
      "\"pos\": {}, \"event\": {}, \"type\": \"request\", ",
      position,
      output::format_event("headers")
    );

    if chunked {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": \"chunked\"",
          heading, method, url, protocol, version,
        ),
        from,
        size,
      );
    } else if content_length > 0 {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": {}",
          heading, method, url, protocol, version, content_length
        ),
        from,
        size,
      );
    } else {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": null",
          heading, method, url, protocol, version,
        ),
        from,
        size,
      );
    }
  }
}

pub fn on_upgrade(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "upgrade", parser.position, from, size);
}

pub fn on_chunk_length(parser: &mut Parser, from: usize, size: usize) {
  output::show_span(parser, "chunk_length", from, size);
}

pub fn on_chunk_extension_name(parser: &mut Parser, from: usize, size: usize) {
  output::show_span(parser, "chunk_extensions_name", from, size);
}

pub fn on_chunk_extension_value(parser: &mut Parser, from: usize, size: usize) {
  output::show_span(parser, "chunk_extension_value", from, size);
}

pub fn on_chunk(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "chunk", parser.position, from, size);
}

pub fn on_data(parser: &mut Parser, from: usize, size: usize) { output::show_span(parser, "data", from, size); }

pub fn on_body(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "body", parser.position, from, size);
}

pub fn on_trailer_name(parser: &mut Parser, from: usize, size: usize) {
  output::show_span(parser, "trailer_name", from, size);
}

pub fn on_trailer_value(parser: &mut Parser, from: usize, size: usize) {
  output::show_span(parser, "trailer_value", from, size);
}

pub fn on_trailers(parser: &mut Parser, from: usize, size: usize) {
  output::event(parser, "trailers", parser.position, from, size);
}
