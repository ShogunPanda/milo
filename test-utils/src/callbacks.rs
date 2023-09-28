#[path = "./context.rs"]
mod context;
mod output;

use std::{slice, str};

use milo::{Parser, ALL_CALLBACKS, DEBUG, RESPONSE};

pub fn before_state_change(parser: &Parser, from: usize, size: usize) -> isize {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"before_state_change\", \"current_state\": \"{}\"",
      parser.position.get(),
      parser.state_string()
    ),
    from,
    size,
  )
}

pub fn after_state_change(parser: &Parser, from: usize, size: usize) -> isize {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"after_state_change\", \"current_state\": \"{}\"",
      parser.position.get(),
      parser.state_string()
    ),
    from,
    size,
  )
}

pub fn on_message_start(parser: &Parser, from: usize, size: usize) -> isize {
  output::append_output(
    parser,
    format!(
      "\"pos\": {}, \"event\": \"begin\", \"configuration\": {{ \"debug\": {}, \"all-callbacks\": {} }}",
      parser.position.get(),
      DEBUG,
      ALL_CALLBACKS
    ),
    from,
    size,
  )
}

pub fn on_message_complete(parser: &Parser, from: usize, size: usize) -> isize {
  output::event(parser, "complete", from, size)
}

pub fn on_error(parser: &Parser, from: usize, size: usize) -> isize {
  unsafe {
    output::append_output(
      parser,
      format!(
        "\"pos\": {}, \"event\": {}, \"error_code={}, \"error_code_string\": \"{}\", reason=\"{}\"",
        parser.position.get(),
        "error",
        parser.error_code.get() as usize,
        parser.error_code_string(),
        str::from_utf8_unchecked(slice::from_raw_parts(
          parser.error_description.get(),
          parser.error_description_len.get()
        ))
      ),
      from,
      size,
    )
  }
}

pub fn on_finish(parser: &Parser, from: usize, size: usize) -> isize { output::event(parser, "finish", from, size) }

pub fn on_request(parser: &Parser, from: usize, size: usize) -> isize { output::event(parser, "request", from, size) }

pub fn on_response(parser: &Parser, from: usize, size: usize) -> isize { output::event(parser, "response", from, size) }

pub fn on_method(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "method", from, size) }

pub fn on_url(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "url", from, size) }

pub fn on_protocol(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "protocol", from, size)
}

pub fn on_version(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "version", from, size)
}

pub fn on_status(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "status", from, size) }

pub fn on_reason(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "reason", from, size) }

pub fn on_header_name(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "header_name", from, size)
}

pub fn on_header_value(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "header_value", from, size)
}

pub fn on_headers(parser: &Parser, from: usize, size: usize) -> isize {
  let context = unsafe { Box::from_raw(parser.owner.get() as *mut context::Context) };

  let position = parser.position.get();
  let chunked = parser.has_chunked_transfer_encoding.get();
  let content_length = parser.content_length.get();
  let method = context.method.clone();
  let url = context.url.clone();
  let protocol = context.protocol.clone();
  let version = context.version.clone();

  Box::into_raw(context);

  if parser.message_type.get() == RESPONSE {
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
          heading,
          parser.status.get(),
          protocol,
          version,
        ),
        from,
        size,
      )
    } else if content_length > 0 {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": {}",
          heading,
          parser.status.get(),
          protocol,
          version,
          content_length
        ),
        from,
        size,
      )
    } else {
      output::append_output(
        parser,
        format!(
          "{}\"status\": {}, \"protocol\": \"{}\", \"version\": \"{}\", \"body\": null",
          heading,
          parser.status.get(),
          protocol,
          version,
        ),
        from,
        size,
      )
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
      )
    } else if content_length > 0 {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": {}",
          heading, method, url, protocol, version, content_length
        ),
        from,
        size,
      )
    } else {
      output::append_output(
        parser,
        format!(
          "{}\"method\": \"{}\", \"url\": \"{}\", \"protocol\": \"{}\", \"version\": \"{}\", \"body\": null",
          heading, method, url, protocol, version,
        ),
        from,
        size,
      )
    }
  }
}

pub fn on_upgrade(parser: &Parser, from: usize, size: usize) -> isize { output::event(parser, "upgrade", from, size) }

pub fn on_chunk_length(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "chunk_length", from, size)
}

pub fn on_chunk_extension_name(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "chunk_extensions_name", from, size)
}

pub fn on_chunk_extension_value(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "chunk_extension_value", from, size)
}

pub fn on_body(parser: &Parser, from: usize, size: usize) -> isize { output::event(parser, "body", from, size) }

pub fn on_data(parser: &Parser, from: usize, size: usize) -> isize { output::show_span(parser, "data", from, size) }

pub fn on_trailer_name(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "trailer_name", from, size)
}

pub fn on_trailer_value(parser: &Parser, from: usize, size: usize) -> isize {
  output::show_span(parser, "trailer_value", from, size)
}

pub fn on_trailers(parser: &Parser, from: usize, size: usize) -> isize { output::event(parser, "trailers", from, size) }
